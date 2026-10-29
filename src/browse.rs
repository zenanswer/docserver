use std::path::Path as FsPath;

use axum::response::{Html, IntoResponse, Response};

use crate::{ui, App};

const ICON_FOLDER: &str = r#"<svg class="ico ico-dir" viewBox="0 0 16 16" aria-hidden="true"><path d="M1.75 2A1.75 1.75 0 0 0 0 3.75v8.5C0 13.216.784 14 1.75 14h12.5A1.75 1.75 0 0 0 16 12.25v-6.5A1.75 1.75 0 0 0 14.25 4H8.06L6.72 2.66A1.75 1.75 0 0 0 5.48 2Z"/></svg>"#;

const ICON_FILE: &str = r#"<svg class="ico ico-file" viewBox="0 0 16 16" aria-hidden="true"><path d="M3 1.75C3 .784 3.784 0 4.75 0h4.19c.464 0 .909.184 1.237.513l2.31 2.31c.33.328.513.773.513 1.237v10.19A1.75 1.75 0 0 1 11.25 16h-6.5A1.75 1.75 0 0 1 3 14.25Zm6.5.75v2.5c0 .414.336.75.75.75h2.5Z"/></svg>"#;

const ICON_IMAGE: &str = r#"<svg class="ico ico-image" viewBox="0 0 16 16" aria-hidden="true"><path d="M1.75 1A1.75 1.75 0 0 0 0 2.75v10.5C0 14.216.784 15 1.75 15h12.5A1.75 1.75 0 0 0 16 13.25V2.75A1.75 1.75 0 0 0 14.25 1ZM5.25 4a1.75 1.75 0 1 1 0 3.5 1.75 1.75 0 0 1 0-3.5Zm8.25 9.25H2.5l3.4-4.6 2.2 2.7 1.8-2.1Z"/></svg>"#;

const ICON_MD: &str = r#"<svg class="ico ico-md" viewBox="0 0 16 16" aria-hidden="true"><path d="M4.75 0A1.75 1.75 0 0 0 3 1.75v12.5C3 15.216 3.784 16 4.75 16h6.5A1.75 1.75 0 0 0 13 14.25V1.75A1.75 1.75 0 0 0 11.25 0Zm.75 11V5h1.3l1.45 1.85L9.7 5h1.3v6h-1.3V7.4L8.25 9.45 6.8 7.4V11Z"/></svg>"#;

const ICON_HTML: &str = r#"<svg class="ico ico-html" viewBox="0 0 16 16" aria-hidden="true"><path d="M4.75 0A1.75 1.75 0 0 0 3 1.75v12.5C3 15.216 3.784 16 4.75 16h6.5A1.75 1.75 0 0 0 13 14.25V1.75A1.75 1.75 0 0 0 11.25 0Zm1.1 5.3 1.05.95L5.6 8l1.3 1.75-1.05.95L3.9 8Zm4.3 0L12.1 8l-1.95 2.7-1.05-.95L10.4 8 9.1 6.25Z"/></svg>"#;

struct Entry {
    name: String,
    is_dir: bool,
    size: u64,
    mtime: String,
}

pub async fn render(app: &App, dir: &FsPath, rel: &str) -> Response {
    let mut entries: Vec<Entry> = Vec::new();

    if let Ok(read_dir) = std::fs::read_dir(dir) {
        for item in read_dir.flatten() {
            let name = item.file_name().to_string_lossy().to_string();
            if name.starts_with('.') && !app.show_hidden {
                continue;
            }
            // 跟随符号链接，但链接到根目录之外的一律不列出来
            let Ok(path) = std::fs::canonicalize(item.path()) else {
                continue;
            };
            if !path.starts_with(app.root.as_ref()) {
                continue;
            }
            let Ok(meta) = std::fs::metadata(&path) else {
                continue;
            };
            entries.push(Entry {
                name,
                is_dir: meta.is_dir(),
                size: meta.len(),
                mtime: meta.modified().map(ui::mtime).unwrap_or_else(|_| "—".into()),
            });
        }
    }

    entries.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| ui::natural_cmp(&a.name, &b.name))
    });

    let mut rows = String::new();
    for entry in &entries {
        let child_rel = format!("{}{}", rel, entry.name);
        let href = if entry.is_dir {
            format!("/{}/", ui::enc_path(&child_rel))
        } else if ui::is_image(FsPath::new(&entry.name)) {
            format!("/__view/{}", ui::enc_path(&child_rel))
        } else {
            format!("/{}", ui::enc_path(&child_rel))
        };
        let icon = if entry.is_dir {
            ICON_FOLDER
        } else if ui::is_image(FsPath::new(&entry.name)) {
            ICON_IMAGE
        } else if ui::is_markdown(FsPath::new(&entry.name)) {
            ICON_MD
        } else if ui::is_html(FsPath::new(&entry.name)) {
            ICON_HTML
        } else {
            ICON_FILE
        };
        let name = if entry.is_dir {
            format!("{}/", entry.name)
        } else {
            entry.name.clone()
        };
        let size = if entry.is_dir {
            String::new()
        } else {
            ui::human_size(entry.size)
        };
        rows.push_str(&format!(
            r#"<tr data-name="{key}"><td class="c-name"><a href="{href}"><span class="ico">{icon}</span>{name}</a></td><td class="c-size">{size}</td><td class="c-time">{mtime}</td></tr>"#,
            key = ui::esc(&entry.name.to_lowercase()),
            href = href,
            icon = icon,
            name = ui::esc(&name),
            size = size,
            mtime = ui::esc(&entry.mtime),
        ));
    }

    let mut crumbs = String::from(r#"<a href="/">根目录</a>"#);
    let mut acc = String::new();
    let segments: Vec<&str> = rel.split('/').filter(|s| !s.is_empty()).collect();
    for (i, seg) in segments.iter().enumerate() {
        acc.push_str(seg);
        acc.push('/');
        if i + 1 == segments.len() {
            crumbs.push_str(&format!(
                r#"<span class="sep">/</span><span class="current">{}</span>"#,
                ui::esc(seg)
            ));
        } else {
            crumbs.push_str(&format!(
                r#"<span class="sep">/</span><a href="/{}">{}</a>"#,
                ui::enc_path(&acc),
                ui::esc(seg)
            ));
        }
    }

    let title = if segments.is_empty() {
        "根目录".to_string()
    } else {
        segments.last().map(|s| s.to_string()).unwrap_or_default()
    };

    let parent_row = if segments.is_empty() {
        String::new()
    } else {
        let parent = if segments.len() == 1 {
            "/".to_string()
        } else {
            format!("/{}/", ui::enc_path(&segments[..segments.len() - 1].join("/")))
        };
        format!(
            r#"<tr class="row-parent"><td class="c-name"><a href="{parent}"><span class="ico ico-up">↩</span>..</a></td><td class="c-size"></td><td class="c-time"></td></tr>"#
        )
    };

    let dir_href = if rel.is_empty() {
        "/".to_string()
    } else {
        format!("/{}", ui::enc_path(rel))
    };
    let index_hint = if dir.join("index.html").is_file() {
        format!(
            r#"<div class="hint"><span>这个目录里有 <code>index.html</code></span><a class="btn" href="{dir_href}index.html">打开页面</a></div>"#
        )
    } else {
        String::new()
    };

    let body = format!(
        r#"<main class="wrap">
<nav class="crumbs">{crumbs}</nav>
<div class="list-head">
  <h1>{title}</h1>
  <input id="filter" class="filter" type="search" placeholder="过滤文件名…" autocomplete="off">
  <span class="count">{count} 项</span>
</div>
{index_hint}
<table class="listing">
<thead><tr><th class="c-name">名称</th><th class="c-size">大小</th><th class="c-time">修改时间</th></tr></thead>
<tbody>
{parent}{rows}
</tbody>
</table>
</main>"#,
        crumbs = crumbs,
        title = ui::esc(&title),
        count = entries.len(),
        index_hint = index_hint,
        parent = parent_row,
        rows = rows,
    );

    Html(ui::page(&title, &body, "")).into_response()
}