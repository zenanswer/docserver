use axum::{
    extract::Path,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
};

const APP_CSS: &str = include_str!("../assets/app.css");
const APP_JS: &str = include_str!("../assets/app.js");
const VIEWER_JS: &str = include_str!("../assets/viewer.js");

pub async fn handler(Path(name): Path<String>) -> Response {
    let (body, content_type) = match name.as_str() {
        "app.css" => (APP_CSS, "text/css; charset=utf-8"),
        "app.js" => (APP_JS, "text/javascript; charset=utf-8"),
        "viewer.js" => (VIEWER_JS, "text/javascript; charset=utf-8"),
        _ => return StatusCode::NOT_FOUND.into_response(),
    };

    (
        [
            (header::CONTENT_TYPE, content_type),
            (header::CACHE_CONTROL, "no-cache"),
        ],
        body,
    )
        .into_response()
}