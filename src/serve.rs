use std::path::{Path as FsPath, PathBuf};

use axum::{
    extract::{Path, State},
    http::{header, StatusCode},
    response::{IntoResponse, Redirect, Response},
};

use crate::{browse, markdown, ui, App};

pub enum Lookup {
    Found(PathBuf),
    Missing,
    Denied,
}

/// 把 URL 里的相对路径解析成磁盘路径，并保证没有逃出根目录。
pub fn resolve(root: &FsPath, rel: &str) -> Lookup {
    if rel.contains('\0') {
        return Lookup::Missing;
    }
    let joined = root.join(rel);
    match std::fs::canonicalize(&joined) {
        Ok(path) => {
            if path.starts_with(root) {
                Lookup::Found(path)
            } else {
                Lookup::Denied
            }
        }
        Err(_) => Lookup::Missing,
    }
}

pub async fn root_handler(State(app): State<App>) -> Response {
    dispatch(app, String::new()).await
}

pub async fn handler(State(app): State<App>, Path(rel): Path<String>) -> Response {
    dispatch(app, rel).await
}

/// `/__raw/<path>`：不管什么类型，一律返回原始字节。
pub async fn raw_handler(State(app): State<App>, Path(rel): Path<String>) -> Response {
    match resolve(app.root.as_ref(), &rel) {
        Lookup::Found(path) if path.is_file() => raw_file(&path).await,
        Lookup::Denied => forbidden(),
        _ => not_found(),
    }
}

async fn dispatch(app: App, rel: String) -> Response {
    let root = app.root.as_ref();
    match resolve(root, &rel) {
        Lookup::Denied => forbidden(),
        Lookup::Missing => not_found(),
        Lookup::Found(path) => {
            if path.is_dir() {
                if !rel.is_empty() && !rel.ends_with('/') {
                    return Redirect::permanent(&format!("/{}/", ui::enc_path(&rel)))
                        .into_response();
                }
                browse::render(&app, &path, &rel).await
            } else if ui::is_markdown(&path) {
                markdown::render_page(&app, &path, &rel).await
            } else {
                raw_file(&path).await
            }
        }
    }
}

pub async fn raw_file(path: &FsPath) -> Response {
    let bytes = match tokio::fs::read(path).await {
        Ok(b) => b,
        Err(_) => return not_found(),
    };

    let mime = mime_guess::from_path(path).first_or_octet_stream().to_string();
    let content_type = if mime.starts_with("text/")
        || mime.contains("javascript")
        || mime.contains("json")
        || mime == "image/svg+xml"
    {
        format!("{mime}; charset=utf-8")
    } else {
        mime
    };

    (
        [
            (header::CONTENT_TYPE, content_type),
            (header::CACHE_CONTROL, "no-cache".to_string()),
        ],
        bytes,
    )
        .into_response()
}

pub fn not_found() -> Response {
    let body = ui::page(
        "404",
        r#"<main class="wrap"><h1>404</h1><p class="muted">这个路径不存在，或者不是一个文件。</p><p><a href="/">回到根目录</a></p></main>"#,
        "",
    );
    (StatusCode::NOT_FOUND, axum::response::Html(body)).into_response()
}

pub fn forbidden() -> Response {
    let body = ui::page(
        "403",
        r#"<main class="wrap"><h1>403</h1><p class="muted">这个路径在服务目录之外。</p><p><a href="/">回到根目录</a></p></main>"#,
        "",
    );
    (StatusCode::FORBIDDEN, axum::response::Html(body)).into_response()
}