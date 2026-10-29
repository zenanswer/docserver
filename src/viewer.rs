use axum::{
    extract::{Path, State},
    response::{Html, IntoResponse, Response},
};

use crate::{serve, ui, App};

pub async fn handler(State(app): State<App>, Path(rel): Path<String>) -> Response {
    let path = match serve::resolve(app.root.as_ref(), &rel) {
        serve::Lookup::Found(p) => p,
        serve::Lookup::Denied => return serve::forbidden(),
        serve::Lookup::Missing => return serve::not_found(),
    };

    if !path.is_file() {
        return serve::not_found();
    }
    if !ui::is_image(&path) {
        return serve::raw_file(&path).await;
    }

    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let dir_rel = match rel.rfind('/') {
        Some(i) => &rel[..=i],
        None => "",
    };
    let dir_abs = path.parent().unwrap_or(app.root.as_ref());

    let mut siblings: Vec<String> = std::fs::read_dir(dir_abs)
        .map(|rd| {
            rd.flatten()
                .filter(|e| ui::is_image(&e.path()))
                .map(|e| e.file_name().to_string_lossy().to_string())
                .collect()
        })
        .unwrap_or_default();
    siblings.sort_by(|a, b| ui::natural_cmp(a, b));

    let index = siblings.iter().position(|s| *s == name).unwrap_or(0);
    let total = siblings.len();

    let prev = if total > 1 {
        let i = if index == 0 { total - 1 } else { index - 1 };
        format!(
            r#"<a class="nav-btn" href="/__view/{}{}" title="上一张（←）">←</a>"#,
            ui::enc_path(dir_rel),
            ui::enc_path(&siblings[i])
        )
    } else {
        r#"<span class="nav-btn disabled">←</span>"#.to_string()
    };
    let next = if total > 1 {
        let i = if index + 1 == total { 0 } else { index + 1 };
        format!(
            r#"<a class="nav-btn" href="/__view/{}{}" title="下一张（→）">→</a>"#,
            ui::enc_path(dir_rel),
            ui::enc_path(&siblings[i])
        )
    } else {
        r#"<span class="nav-btn disabled">→</span>"#.to_string()
    };

    let body = format!(
        r#"<main class="viewer">
<div class="viewer-bar">
  {prev}
  <span class="viewer-name" title="{name}">{name}</span>
  <span class="viewer-idx">{idx} / {total}</span>
  <span class="spacer"></span>
  <span class="viewer-scale" id="scale">100%</span>
  <button class="icon-btn" data-zoom="out" title="缩小（-）">−</button>
  <button class="icon-btn" data-zoom="in" title="放大（+）">+</button>
  <button class="btn" data-zoom="fit" title="适应窗口（0）">适应</button>
  <button class="btn" data-zoom="one" title="原始大小（1）">1:1</button>
  {next}
</div>
<div class="stage" id="stage">
  <img id="pic" src="/{src}" alt="{name}" draggable="false">
</div>
<script src="/__asset/viewer.js"></script>
</main>"#,
        prev = prev,
        next = next,
        name = ui::esc(&name),
        idx = index + 1,
        total = total,
        src = ui::enc_path(&rel),
    );

    Html(ui::page(&name, &body, "")).into_response()
}