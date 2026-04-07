use std::path::{Path, PathBuf};

use axum::extract::Path as AxumPath;
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};

pub async fn panel_index() -> Response {
    serve_path(&dist_dir(), Path::new("index.html"), true)
}

pub async fn panel_asset(AxumPath(path): AxumPath<String>) -> Response {
    let requested = PathBuf::from(path);
    let fallback_to_index = requested.extension().is_none();
    serve_path(&dist_dir(), &requested, fallback_to_index)
}

fn dist_dir() -> PathBuf {
    PathBuf::from("web/dist")
}

fn serve_path(dist_dir: &Path, requested: &Path, fallback_to_index: bool) -> Response {
    let candidate = dist_dir.join(requested);

    if candidate.is_file() {
        return file_response(&candidate);
    }

    if fallback_to_index {
        let index = dist_dir.join("index.html");
        if index.is_file() {
            return file_response(&index);
        }
    }

    (
        StatusCode::NOT_FOUND,
        "admin panel assets are not built yet".to_string(),
    )
        .into_response()
}

fn file_response(path: &Path) -> axum::response::Response {
    match std::fs::read(path) {
        Ok(body) => {
            let mut headers = HeaderMap::new();
            headers.insert(header::CONTENT_TYPE, content_type(path));
            (StatusCode::OK, headers, body).into_response()
        }
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("failed to read panel asset: {error}"),
        )
            .into_response(),
    }
}

fn content_type(path: &Path) -> header::HeaderValue {
    let value = match path
        .extension()
        .and_then(|item| item.to_str())
        .unwrap_or_default()
    {
        "html" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" => "application/javascript; charset=utf-8",
        "json" => "application/json; charset=utf-8",
        "svg" => "image/svg+xml",
        _ => "application/octet-stream",
    };
    header::HeaderValue::from_static(value)
}
