mod auth;
mod filemanager;
mod static_files;
mod stats;
mod user;

use std::path::PathBuf;
use std::sync::LazyLock;

use axum::Router;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool};
use tower_http::cors::{Any, CorsLayer};
use utoipa::OpenApi;
use utoipa_axum::{router::OpenApiRouter, routes};
use utoipa_swagger_ui::SwaggerUi;

static KEYS: LazyLock<auth::Keys> = LazyLock::new(|| {
    let secret = std::env::var("JWT_SECRET").expect("JWT_SECRET must be set");
    auth::Keys::new(secret.as_bytes())
});

#[derive(Clone)]
pub struct AppState {
    pub db_pool: SqlitePool,
    pub storage_root: PathBuf,
}

#[derive(OpenApi)]
#[openapi(
    paths(
        auth::signup,
        auth::login,
        auth::me,
        filemanager::get_files_handler,
        filemanager::upload_file,
        filemanager::download_file,
        filemanager::delete_file,
        stats::get_stats
    ),
    components(
        schemas(
            auth::Claims,
            auth::AuthBody,
            auth::LoginRequest,
            user::CreateUserRequest,
            user::UserResponse,
            filemanager::FileQuery,
            filemanager::FileResponse,
            filemanager::FileMetadata,
            stats::SystemStats
        )
    ),
    tags(
        (name = "auth", description = "Authentication endpoints"),
        (name = "files", description = "File management endpoints"),
        (name = "stats", description = "System statistics endpoints")
    ),
    modifiers(&SecurityAddon)
)]
struct ApiDoc;

struct SecurityAddon;

impl utoipa::Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearer_auth",
                utoipa::openapi::security::SecurityScheme::Http(
                    utoipa::openapi::security::HttpBuilder::new()
                        .scheme(utoipa::openapi::security::HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .build(),
                ),
            );
        }
    }
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let storage_root = std::env::var("STORAGE_ROOT").unwrap_or_else(|_| "./storage".to_string());
    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse::<u16>()
        .expect("PORT must be a valid number");

    // Configure SQLite connection to create database if missing
    let connect_options = database_url
        .parse::<SqliteConnectOptions>()
        .expect("Invalid DATABASE_URL")
        .create_if_missing(true);

    let db_pool = SqlitePool::connect_with(connect_options)
        .await
        .expect("Failed to connect to database");

    // Run migrations to set up the schema
    sqlx::migrate!("./migrations")
        .run(&db_pool)
        .await
        .expect("Failed to run migrations");

    tokio::fs::create_dir_all(&storage_root)
        .await
        .expect("Failed to create storage root directory");

    let state = AppState {
        db_pool,
        storage_root: PathBuf::from(storage_root),
    };

    let (router, api) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .routes(routes!(auth::signup))
        .routes(routes!(auth::login))
        .routes(routes!(auth::me))
        .routes(routes!(filemanager::get_files_handler))
        .routes(routes!(filemanager::upload_file))
        .routes(routes!(filemanager::download_file))
        .routes(routes!(filemanager::delete_file))
        .routes(routes!(stats::get_stats))
        .with_state(state)
        .split_for_parts();

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .merge(router)
        .merge(SwaggerUi::new("/swagger-ui").url("/api/openapi.json", api))
        .layer(cors)
        .fallback(static_files::handler);

    let addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    println!("Server running on http://localhost:{}", port);
    println!("OpenAPI spec: http://localhost:{}/api/openapi.json", port);
    println!("Swagger UI: http://localhost:{}/swagger-ui", port);
    axum::serve(listener, app).await.unwrap();
}
