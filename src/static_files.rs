use axum::{
    body::Body,
    http::{StatusCode, Uri, header},
    response::IntoResponse,
};
use rust_embed::Embed;

#[derive(Embed)]
#[folder = "frontend/dist"]
pub struct Asset;

pub async fn handler(uri: Uri) -> impl IntoResponse {
    let path = uri.path().trim_start_matches('/');

    if let Some(content) = Asset::get(path) {
        let mime = mime_guess::from_path(path).first_or_octet_stream();
        return (
            [(header::CONTENT_TYPE, mime.as_ref())],
            Body::from(content.data),
        )
            .into_response();
    }

    match Asset::get("index.html") {
        Some(content) => (
            [(header::CONTENT_TYPE, "text/html")],
            Body::from(content.data),
        )
            .into_response(),
        None => (StatusCode::NOT_FOUND, "404 Not Found").into_response(),
    }
}
