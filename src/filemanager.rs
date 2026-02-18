use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
};
use axum_typed_multipart::{FieldData, TryFromMultipart, TypedMultipart};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{FromRow, SqlitePool};
use tokio::io::AsyncWriteExt;
use tokio_util::io::ReaderStream;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::AppState;
use crate::auth::Claims;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct File {
    pub id: String,
    pub user_id: String,
    pub original_name: String,
    pub mime_type: String,
    pub size_bytes: i64,
    pub is_encrypted: bool,
    pub storage_path: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FileMetadata {
    pub original_name: String,
    pub mime_type: String,
    pub size_bytes: i64,
    pub client_encryption_algo: String,
}

#[derive(Debug, TryFromMultipart)]
pub struct FileUploadForm {
    #[form_data(limit = "100MB")]
    pub file: FieldData<axum::body::Bytes>,
    pub metadata: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct FileResponse {
    pub id: String,
    pub original_name: String,
    pub mime_type: String,
    pub size_bytes: i64,
    pub created_at: String,
}

impl From<File> for FileResponse {
    fn from(file: File) -> Self {
        Self {
            id: file.id,
            original_name: file.original_name,
            mime_type: file.mime_type,
            size_bytes: file.size_bytes,
            created_at: file.created_at,
        }
    }
}

#[derive(Deserialize, ToSchema, IntoParams)]
pub struct FileQuery {
    pub q: Option<String>,
    pub sort: Option<String>,
    pub direction: Option<String>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct FileListResponse {
    pub files: Vec<FileResponse>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
    pub total_pages: i64,
}

#[derive(Debug)]
pub enum FileError {
    DatabaseError(sqlx::Error),
    NotFound,
    Unauthorized,
    StorageError,
    InvalidMetadata,
    InternalError,
}

impl IntoResponse for FileError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            FileError::DatabaseError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Database error"),
            FileError::NotFound => (StatusCode::NOT_FOUND, "File not found"),
            FileError::Unauthorized => (StatusCode::FORBIDDEN, "You don't own this file"),
            FileError::StorageError => (StatusCode::INTERNAL_SERVER_ERROR, "Storage error"),
            FileError::InvalidMetadata => (StatusCode::BAD_REQUEST, "Invalid metadata"),
            FileError::InternalError => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error")
            }
        };
        let body = Json(json!({
            "error": error_message,
        }));
        (status, body).into_response()
    }
}

pub struct FileRepository {
    pool: SqlitePool,
}

impl FileRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create_file(&self, file: &File) -> Result<(), FileError> {
        sqlx::query(
            "INSERT INTO files (id, user_id, original_name, mime_type, size_bytes, is_encrypted, storage_path, created_at) 
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&file.id)
        .bind(&file.user_id)
        .bind(&file.original_name)
        .bind(&file.mime_type)
        .bind(file.size_bytes)
        .bind(file.is_encrypted)
        .bind(&file.storage_path)
        .bind(&file.created_at)
        .execute(&self.pool)
        .await
        .map_err(FileError::DatabaseError)?;

        Ok(())
    }

    pub async fn list_files(
        &self,
        user_id: &str,
        search_query: Option<&str>,
        sort: Option<&str>,
        direction: Option<&str>,
        page: i64,
        page_size: i64,
    ) -> Result<Vec<File>, FileError> {
        let mut query = String::from("SELECT * FROM files WHERE user_id = ?");

        if search_query.is_some() {
            query.push_str(" AND original_name LIKE ?");
        }

        let sort_field = match sort {
            Some("size") => "size_bytes",
            Some("date") => "created_at",
            _ => "original_name",
        };

        let sort_dir = match direction {
            Some("desc") => "DESC",
            _ => "ASC",
        };

        query.push_str(&format!(" ORDER BY {} {}", sort_field, sort_dir));
        
        let offset = (page - 1) * page_size;
        query.push_str(&format!(" LIMIT {} OFFSET {}", page_size, offset));

        let mut query_builder = sqlx::query_as::<_, File>(&query).bind(user_id);

        if let Some(q) = search_query {
            query_builder = query_builder.bind(format!("%{}%", q));
        }

        query_builder
            .fetch_all(&self.pool)
            .await
            .map_err(FileError::DatabaseError)
    }

    pub async fn get_file(&self, id: &str, user_id: &str) -> Result<Option<File>, FileError> {
        sqlx::query_as::<_, File>("SELECT * FROM files WHERE id = ? AND user_id = ?")
            .bind(id)
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(FileError::DatabaseError)
    }

    pub async fn count_files(
        &self,
        user_id: &str,
        search_query: Option<&str>,
    ) -> Result<i64, FileError> {
        let mut query = String::from("SELECT COUNT(*) as count FROM files WHERE user_id = ?");

        if search_query.is_some() {
            query.push_str(" AND original_name LIKE ?");
        }

        let mut query_builder = sqlx::query_scalar::<_, i64>(&query).bind(user_id);

        if let Some(q) = search_query {
            query_builder = query_builder.bind(format!("%{}%", q));
        }

        query_builder
            .fetch_one(&self.pool)
            .await
            .map_err(FileError::DatabaseError)
    }

    pub async fn delete_file(&self, id: &str, user_id: &str) -> Result<bool, FileError> {
        let result = sqlx::query("DELETE FROM files WHERE id = ? AND user_id = ?")
            .bind(id)
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(FileError::DatabaseError)?;

        Ok(result.rows_affected() > 0)
    }
}

#[utoipa::path(
    post,
    path = "/api/files/upload",
    tag = "files",
    responses(
        (status = 201, description = "File uploaded successfully", body = FileResponse),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn upload_file(
    claims: Claims,
    State(state): State<AppState>,
    TypedMultipart(form): TypedMultipart<FileUploadForm>,
) -> Result<(StatusCode, Json<FileResponse>), FileError> {
    let metadata: FileMetadata =
        serde_json::from_str(&form.metadata).map_err(|_| FileError::InvalidMetadata)?;

    let file_id = Uuid::new_v4().to_string();
    let storage_path = format!("{}/{}.bin", claims.user_id, file_id);
    let full_path = state.storage_root.join(&storage_path);

    let mut file_handle = tokio::fs::File::create(&full_path)
        .await
        .map_err(|_| FileError::StorageError)?;

    file_handle
        .write_all(&form.file.contents)
        .await
        .map_err(|_| FileError::StorageError)?;

    file_handle
        .flush()
        .await
        .map_err(|_| FileError::StorageError)?;

    // Calculate actual file size from uploaded data to prevent size spoofing
    let actual_size = form.file.contents.len() as i64;

    let file = File {
        id: file_id.clone(),
        user_id: claims.user_id.clone(),
        original_name: metadata.original_name,
        mime_type: metadata.mime_type,
        size_bytes: actual_size, // Use actual size, not user-provided metadata
        is_encrypted: true,
        storage_path,
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    let file_repo = FileRepository::new(state.db_pool);
    file_repo.create_file(&file).await?;

    Ok((StatusCode::CREATED, Json(file.into())))
}

#[utoipa::path(
    get,
    path = "/api/files",
    tag = "files",
    params(FileQuery),
    responses(
        (status = 200, description = "Files retrieved successfully", body = FileListResponse),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn get_files_handler(
    claims: Claims,
    State(state): State<AppState>,
    Query(query): Query<FileQuery>,
) -> Result<Json<FileListResponse>, FileError> {
    let file_repo = FileRepository::new(state.db_pool.clone());

    let page = query.page.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(20).max(1).min(100);

    let total = file_repo
        .count_files(&claims.user_id, query.q.as_deref())
        .await?;

    let files = file_repo
        .list_files(
            &claims.user_id,
            query.q.as_deref(),
            query.sort.as_deref(),
            query.direction.as_deref(),
            page,
            page_size,
        )
        .await?;

    let total_pages = (total as f64 / page_size as f64).ceil() as i64;
    let responses: Vec<FileResponse> = files.into_iter().map(|f| f.into()).collect();

    Ok(Json(FileListResponse {
        files: responses,
        total,
        page,
        page_size,
        total_pages,
    }))
}

#[utoipa::path(
    get,
    path = "/api/files/{id}/download",
    tag = "files",
    params(
        ("id" = String, Path, description = "File ID")
    ),
    responses(
        (status = 200, description = "File download", content_type = "application/octet-stream"),
        (status = 404, description = "File not found"),
        (status = 403, description = "Unauthorized")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn download_file(
    claims: Claims,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Response, FileError> {
    let file_repo = FileRepository::new(state.db_pool.clone());

    let file = file_repo
        .get_file(&id, &claims.user_id)
        .await?
        .ok_or(FileError::NotFound)?;

    let full_path = state.storage_root.join(&file.storage_path);

    let file_handle = tokio::fs::File::open(&full_path)
        .await
        .map_err(|_| FileError::StorageError)?;

    let stream = ReaderStream::new(file_handle);
    let body = axum::body::Body::from_stream(stream);

    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        "application/octet-stream".parse().unwrap(),
    );
    headers.insert(
        header::CONTENT_DISPOSITION,
        format!("attachment; filename=\"{}\"", file.original_name)
            .parse()
            .unwrap(),
    );

    Ok((headers, body).into_response())
}

#[utoipa::path(
    delete,
    path = "/api/files/{id}",
    tag = "files",
    params(
        ("id" = String, Path, description = "File ID")
    ),
    responses(
        (status = 204, description = "File deleted successfully"),
        (status = 404, description = "File not found"),
        (status = 403, description = "Unauthorized")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn delete_file(
    claims: Claims,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, FileError> {
    let file_repo = FileRepository::new(state.db_pool.clone());

    let file = file_repo
        .get_file(&id, &claims.user_id)
        .await?
        .ok_or(FileError::NotFound)?;

    let full_path = state.storage_root.join(&file.storage_path);

    tokio::fs::remove_file(&full_path)
        .await
        .map_err(|_| FileError::StorageError)?;

    file_repo.delete_file(&id, &claims.user_id).await?;

    Ok(StatusCode::NO_CONTENT)
}
