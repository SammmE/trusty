use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use sysinfo::{System, Disks, Networks};
use utoipa::ToSchema;
use std::time::{Duration, Instant};

use crate::{AppState, auth::Claims};

pub struct StatsCache {
    sys: System,
    last_refresh: Instant,
    cache_duration: Duration,
}

impl StatsCache {
    pub fn new() -> Self {
        Self {
            sys: System::new_all(),
            last_refresh: Instant::now() - Duration::from_secs(10), // Force initial refresh
            cache_duration: Duration::from_millis(500), // Refresh at most every 500ms (2Hz)
        }
    }

    pub fn refresh_if_needed(&mut self) {
        if self.last_refresh.elapsed() >= self.cache_duration {
            self.sys.refresh_all();
            self.last_refresh = Instant::now();
        }
    }

    pub fn get_system(&self) -> &System {
        &self.sys
    }
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct SystemStats {
    /// CPU usage percentage
    pub cpu_usage: f32,
    /// Memory used in bytes
    pub memory_used: u64,
    /// Total memory in bytes
    pub memory_total: u64,
    /// Memory usage percentage
    pub memory_percent: f32,
    /// Disk used in bytes
    pub disk_used: u64,
    /// Total disk space in bytes
    pub disk_total: u64,
    /// Disk usage percentage
    pub disk_percent: f32,
    /// Network received bytes
    pub network_rx: u64,
    /// Network transmitted bytes
    pub network_tx: u64,
    /// Total files stored
    pub total_files: i64,
    /// Total storage used by files
    pub total_file_size: i64,
    /// Uptime in seconds
    pub uptime: u64,
    /// Update rate in Hz
    pub update_rate_hz: u32,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct StatsConfig {
    /// Update rate in Hz (default: 50)
    pub update_rate_hz: u32,
}

/// Get system statistics
#[utoipa::path(
    get,
    path = "/api/stats",
    responses(
        (status = 200, description = "System statistics", body = SystemStats),
        (status = 401, description = "Unauthorized")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn get_stats(
    claims: Claims,
    State(state): State<AppState>,
) -> Result<Json<SystemStats>, StatusCode> {
    // Refresh stats cache (throttled to prevent DoS) and collect stats
    // We need to drop the lock before any await points
    let (cpu_usage, memory_used, memory_total, disk_used, disk_total, network_rx, network_tx, uptime) = {
        let mut cache = state.stats_cache.lock().unwrap();
        cache.refresh_if_needed();
        let sys = cache.get_system();

        // Get CPU usage
        let cpu_usage = sys.global_cpu_usage();

        // Get memory stats
        let memory_used = sys.used_memory();
        let memory_total = sys.total_memory();

        // Get disk stats
        let disks = Disks::new_with_refreshed_list();
        let (disk_used, disk_total) = disks.iter().fold((0u64, 0u64), |(used, total), disk| {
            (used + (disk.total_space() - disk.available_space()), total + disk.total_space())
        });

        // Get network stats
        let networks = Networks::new_with_refreshed_list();
        let (network_rx, network_tx) = networks.iter().fold((0u64, 0u64), |(rx, tx), (_name, network)| {
            (rx + network.total_received(), tx + network.total_transmitted())
        });

        let uptime = System::uptime();

        // Drop the lock before the database query
        drop(cache);

        (cpu_usage, memory_used, memory_total, disk_used, disk_total, network_rx, network_tx, uptime)
    };

    // Calculate percentages
    let memory_percent = if memory_total > 0 {
        (memory_used as f32 / memory_total as f32) * 100.0
    } else {
        0.0
    };

    let disk_percent = if disk_total > 0 {
        (disk_used as f32 / disk_total as f32) * 100.0
    } else {
        0.0
    };

    // Get file stats from database - SCOPED TO CURRENT USER
    let file_stats: (i64, i64) = sqlx::query_as(
        "SELECT COUNT(*), COALESCE(SUM(size_bytes), 0) FROM files WHERE user_id = ?"
    )
    .bind(&claims.user_id)
    .fetch_one(&state.db_pool)
    .await
    .unwrap_or((0, 0));

    Ok(Json(SystemStats {
        cpu_usage,
        memory_used,
        memory_total,
        memory_percent,
        disk_used,
        disk_total,
        disk_percent,
        network_rx,
        network_tx,
        total_files: file_stats.0,
        total_file_size: file_stats.1,
        uptime,
        update_rate_hz: 2, // Actual refresh rate is 2Hz (every 500ms)
    }))
}
