<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { type ApiError } from '$lib/api';
	import {
		Dialog,
		DialogContent,
		DialogDescription,
		DialogHeader,
		DialogTitle
	} from '$lib/components/ui/dialog';
	import { Activity } from 'lucide-svelte';

	interface Props {
		open: boolean;
		onClose: () => void;
	}

	let { open = $bindable(), onClose }: Props = $props();

	interface SystemStats {
		cpu_usage: number;
		memory_used: number;
		memory_total: number;
		memory_percent: number;
		disk_used: number;
		disk_total: number;
		disk_percent: number;
		network_rx: number;
		network_tx: number;
		total_files: number;
		total_file_size: number;
		uptime: number;
		update_rate_hz: number;
	}

	let stats = $state<SystemStats | null>(null);
	let error = $state('');
	let intervalId: number | null = null;

	async function fetchStats() {
		try {
			const token = sessionStorage.getItem('auth_token') || localStorage.getItem('auth_token');
			const response = await fetch('http://localhost:3000/api/stats', {
				headers: token ? { Authorization: `Bearer ${token}` } : {},
			});

			if (!response.ok) {
				throw new Error('Failed to fetch stats');
			}

			stats = await response.json();
			error = '';
		} catch (err) {
			error = 'Failed to load system statistics';
			console.error('Stats error:', err);
		}
	}

	function formatBytes(bytes: number): string {
		if (bytes < 1024) return bytes + ' B';
		if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(2) + ' KB';
		if (bytes < 1024 * 1024 * 1024) return (bytes / (1024 * 1024)).toFixed(2) + ' MB';
		if (bytes < 1024 * 1024 * 1024 * 1024) return (bytes / (1024 * 1024 * 1024)).toFixed(2) + ' GB';
		return (bytes / (1024 * 1024 * 1024 * 1024)).toFixed(2) + ' TB';
	}

	function formatUptime(seconds: number): string {
		const days = Math.floor(seconds / 86400);
		const hours = Math.floor((seconds % 86400) / 3600);
		const minutes = Math.floor((seconds % 3600) / 60);
		const secs = Math.floor(seconds % 60);

		if (days > 0) {
			return `${days}d ${hours}h ${minutes}m`;
		} else if (hours > 0) {
			return `${hours}h ${minutes}m ${secs}s`;
		} else if (minutes > 0) {
			return `${minutes}m ${secs}s`;
		} else {
			return `${secs}s`;
		}
	}

	$effect(() => {
		if (open) {
			fetchStats();
			// Update at the configured rate (default 50Hz = 20ms interval)
			const updateInterval = stats?.update_rate_hz ? 1000 / stats.update_rate_hz : 20;
			intervalId = window.setInterval(fetchStats, updateInterval);
		} else {
			if (intervalId !== null) {
				clearInterval(intervalId);
				intervalId = null;
			}
		}

		return () => {
			if (intervalId !== null) {
				clearInterval(intervalId);
			}
		};
	});

	function getProgressColor(percent: number): string {
		if (percent < 50) return 'bg-green-500';
		if (percent < 75) return 'bg-yellow-500';
		if (percent < 90) return 'bg-orange-500';
		return 'bg-red-500';
	}
</script>

<Dialog bind:open>
	<DialogContent class="sm:max-w-[600px]">
		<div class="max-h-[80vh] overflow-auto">
		<DialogHeader>
			<div class="flex items-center gap-2">
				<Activity class="h-5 w-5" />
				<DialogTitle>System Statistics</DialogTitle>
			</div>
			<DialogDescription>
				Real-time system performance and usage metrics
				{#if stats}
					(updating at {stats.update_rate_hz}Hz)
				{/if}
			</DialogDescription>
		</DialogHeader>

		<div class="space-y-6 py-4">
			{#if error}
				<div class="rounded-md bg-red-50 dark:bg-red-900/20 p-4">
					<p class="text-sm text-red-800 dark:text-red-200">{error}</p>
				</div>
			{:else if stats}
				<!-- CPU Usage -->
				<div class="space-y-2">
					<div class="flex items-center justify-between">
						<span class="text-sm font-medium">CPU Usage</span>
						<span class="text-sm text-muted-foreground">{stats.cpu_usage.toFixed(1)}%</span>
					</div>
					<div class="h-2 w-full overflow-hidden rounded-full bg-muted">
						<div
							class="{getProgressColor(stats.cpu_usage)} h-full transition-all duration-300"
							style="width: {Math.min(stats.cpu_usage, 100)}%"
						></div>
					</div>
				</div>

				<!-- Memory Usage -->
				<div class="space-y-2">
					<div class="flex items-center justify-between">
						<span class="text-sm font-medium">Memory</span>
						<span class="text-sm text-muted-foreground">
							{formatBytes(stats.memory_used)} / {formatBytes(stats.memory_total)} ({stats.memory_percent.toFixed(1)}%)
						</span>
					</div>
					<div class="h-2 w-full overflow-hidden rounded-full bg-muted">
						<div
							class="{getProgressColor(stats.memory_percent)} h-full transition-all duration-300"
							style="width: {Math.min(stats.memory_percent, 100)}%"
						></div>
					</div>
				</div>

				<!-- Disk Usage -->
				<div class="space-y-2">
					<div class="flex items-center justify-between">
						<span class="text-sm font-medium">Disk Space</span>
						<span class="text-sm text-muted-foreground">
							{formatBytes(stats.disk_used)} / {formatBytes(stats.disk_total)} ({stats.disk_percent.toFixed(1)}%)
						</span>
					</div>
					<div class="h-2 w-full overflow-hidden rounded-full bg-muted">
						<div
							class="{getProgressColor(stats.disk_percent)} h-full transition-all duration-300"
							style="width: {Math.min(stats.disk_percent, 100)}%"
						></div>
					</div>
				</div>

				<!-- Network Stats -->
				<div class="grid grid-cols-2 gap-4">
					<div class="rounded-lg border bg-card p-4">
						<div class="text-sm font-medium text-muted-foreground">Network RX</div>
						<div class="mt-1 text-2xl font-bold">{formatBytes(stats.network_rx)}</div>
					</div>
					<div class="rounded-lg border bg-card p-4">
						<div class="text-sm font-medium text-muted-foreground">Network TX</div>
						<div class="mt-1 text-2xl font-bold">{formatBytes(stats.network_tx)}</div>
					</div>
				</div>

				<!-- File Storage Stats -->
				<div class="grid grid-cols-2 gap-4">
					<div class="rounded-lg border bg-card p-4">
						<div class="text-sm font-medium text-muted-foreground">Total Files</div>
						<div class="mt-1 text-2xl font-bold">{stats.total_files.toLocaleString()}</div>
					</div>
					<div class="rounded-lg border bg-card p-4">
						<div class="text-sm font-medium text-muted-foreground">Storage Used</div>
						<div class="mt-1 text-2xl font-bold">{formatBytes(stats.total_file_size)}</div>
					</div>
				</div>

				<!-- System Uptime -->
				<div class="rounded-lg border bg-card p-4">
					<div class="text-sm font-medium text-muted-foreground">System Uptime</div>
					<div class="mt-1 text-2xl font-bold">{formatUptime(stats.uptime)}</div>
				</div>
			{:else}
				<div class="flex h-48 items-center justify-center">
					<div class="text-center">
						<div class="mb-4 h-8 w-8 animate-spin rounded-full border-4 border-gray-300 border-t-blue-600 mx-auto"></div>
						<p class="text-sm text-muted-foreground">Loading statistics...</p>
					</div>
				</div>
			{/if}
		</div>
		</div>
	</DialogContent>
</Dialog>
