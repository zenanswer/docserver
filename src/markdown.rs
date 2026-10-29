use std::path::Path as FsPath;

use axum::response::{Html, IntoResponse, Response};
use pulldown_cmark::{html, Options, Parser};

use crate::{ui, App};

struct TocEntry {
    level: u8,
    id: String,
    text: String,
}

pub async fn render_page(_app: &App, path: &FsPath, rel: &str) -> Response {
    let raw = match tokio::fs::read(path).await {
        Ok(bytes) => String::from_utf8_lossy(&bytes).to_string(),
        Err(_) => return crate::serve::not_found(),
    };

    let dir = match rel.rfind('/') {
        Some(i) => &rel[..i],
        None => "",
    };

    let (content, toc) = render_markdown(&raw, dir);
    // 结构太深的文档，TOC 只保留一级标题，避免侧栏被挤满
    let toc: Vec<(u8, String, String)> = if toc.len() > 12 {
        toc.into_iter().filter(|t| t.0 == 2).collect()
    } else {
        toc
    };
    let title = toc
        .first()
        .map(|t| t.2.clone())
        .or_else(|| {
            rel.rsplit('/')
                .next()
                .map(|s| s.trim_end_matches(".md").to_string())
        })
        .unwrap_or_else(|| "文档".to_string());

    let mut crumbs = String::from(r#"<a href="/">根目录</a>"#);
    let mut acc = String::new();
    let segments: Vec<&str> = rel.split('/').filter(|s| !s.is_empty()).collect();
    for (i, seg) in segments.iter().enumerate() {
        acc.push_str(seg);
        if i + 1 == segments.len() {
            crumbs.push_str(&format!(
                r#"<span class="sep">/</span><span class="current">{}</span>"#,
                ui::esc(seg)
            ));
        } else {
            acc.push('/');
            crumbs.push_str(&format!(
                r#"<span class="sep">/</span><a href="/{}">{}</a>"#,
                ui::enc_path(&acc),
                ui::esc(seg)
            ));
        }
    }

    let toc_html = if toc.is_empty() {
        String::new()
    } else {
        let mut items = String::new();
        for entry in &toc {
            items.push_str(&format!(
                r##"<li class="lv{level}"><a href="#{id}">{text}</a></li>"##,
                level = entry.0,
                id = ui::esc(&entry.1),
                text = ui::esc(&entry.2),
            ));
        }
        format!(
            r#"<aside class="toc"><div class="toc-title">本页目录</div><ul>{items}</ul></aside>"#
        )
    };

    let layout_class = if toc_html.is_empty() {
        "layout no-toc"
    } else {
        "layout"
    };

    let body = format!(
        r#"<main class="{layout_class}">
<article class="markdown-body">
<nav class="crumbs">{crumbs}</nav>
{content}
<footer class="doc-foot"><span class="muted">源文件 {rel}</span><a href="/__raw/{enc_rel}">查看原文</a></footer>
</article>
{toc}
</main>"#,
        layout_class = layout_class,
        crumbs = crumbs,
        content = content,
        rel = ui::esc(rel),
        enc_rel = ui::enc_path(rel),
        toc = toc_html,
    );

    Html(ui::page(&title, &body, "")).into_response()
}

pub fn render_markdown(md: &str, dir: &str) -> (String, Vec<(u8, String, String)>) {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_SMART_PUNCTUATION);

    let parser = Parser::new_ext(md, options);
    let mut html_out = String::with_capacity(md.len() * 2);
    html::push_html(&mut html_out, parser);

    let html_out = rewrite_urls(&html_out, dir);
    let (html_out, toc) = add_heading_ids(&html_out);

    (
        html_out,
        toc.into_iter()
            .map(|t| (t.level, t.id, t.text))
            .collect(),
    )
}

fn rewrite_urls(html: &str, dir: &str) -> String {
    let mut out = String::with_capacity(html.len() + 64);
    let mut last = 0usize;
    let mut idx = 0usize;

    while idx < html.len() {
        let rest = &html[idx..];
        let mut hit: Option<(usize, usize)> = None;
        for pattern in ["href=\"", "src=\""] {
            if let Some(offset) = rest.find(pattern) {
                if hit.map_or(true, |(best, _)| offset < best) {
                    hit = Some((offset, pattern.len()));
                }
            }
        }
        let Some((offset, pattern_len)) = hit else {
            break;
        };
        let value_start = idx + offset + pattern_len;
        let Some(rel_end) = html[value_start..].find('"') else {
            break;
        };
        let value_end = value_start + rel_end;
        out.push_str(&html[last..value_start]);
        out.push_str(&rewrite_one(&html[value_start..value_end], dir));
        last = value_end;
        idx = value_end;
    }
    out.push_str(&html[last..]);
    out
}

fn rewrite_one(url: &str, dir: &str) -> String {
    if url.is_empty() || ui::is_external(url) {
        return url.to_string();
    }
    // pulldown-cmark 输出的 href/src 已经是百分号编码过的，先解回原文再重新编码，
    // 否则中文路径会被编码两次（%25E5…）。
    let decoded = percent_encoding::percent_decode_str(url)
        .decode_utf8_lossy()
        .to_string();
    let (path, suffix) = match decoded.find(['#', '?']) {
        Some(i) => (&decoded[..i], &decoded[i..]),
        None => (decoded.as_str(), ""),
    };
    let joined = ui::join_rel(dir, path);
    format!("/{}{}", ui::enc_path(&joined), suffix)
}

fn add_heading_ids(html: &str) -> (String, Vec<TocEntry>) {
    let mut out = String::with_capacity(html.len() + 128);
    let mut toc: Vec<TocEntry> = Vec::new();
    let mut used: Vec<String> = Vec::new();
    let mut cursor = 0usize;
    let mut last = 0usize;

    while cursor < html.len() {
        let rest = &html[cursor..];
        let mut found: Option<(usize, u8)> = None;
        for level in [2u8, 3, 4] {
            let tag = format!("<h{level}>");
            if let Some(offset) = rest.find(&tag) {
                if found.map_or(true, |(best, _)| offset < best) {
                    found = Some((offset, level));
                }
            }
        }
        let Some((offset, level)) = found else {
            break;
        };
        let tag_start = cursor + offset;
        let tag_end = tag_start + 4;
        let close = format!("</h{level}>");
        let Some(close_offset) = html[tag_end..].find(&close) else {
            break;
        };
        let text = strip_tags(&html[tag_end..tag_end + close_offset]);

        let mut id = slugify(&text);
        if id.is_empty() {
            id = format!("section-{level}");
        }
        let mut unique = id.clone();
        let mut n = 2;
        while used.contains(&unique) {
            unique = format!("{id}-{n}");
            n += 1;
        }
        used.push(unique.clone());

        out.push_str(&html[last..tag_start]);
        out.push_str(&format!(r#"<h{level} id="{}">"#, ui::esc(&unique)));
        last = tag_end;
        cursor = tag_end;

        if level <= 3 {
            toc.push(TocEntry {
                level,
                id: unique,
                text,
            });
        }
    }

    out.push_str(&html[last..]);
    (out, toc)
}

fn strip_tags(html: &str) -> String {
    let mut text = String::with_capacity(html.len());
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => text.push(c),
            _ => {}
        }
    }
    text.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .trim()
        .to_string()
}

fn slugify(text: &str) -> String {
    let mut slug = String::new();
    let mut last_dash = true;
    for c in text.chars() {
        if c.is_alphanumeric() {
            for lower in c.to_lowercase() {
                slug.push(lower);
            }
            last_dash = false;
        } else if !last_dash && (c.is_whitespace() || c == '-' || c == '_' || c == '/') {
            slug.push('-');
            last_dash = true;
        }
    }
    slug.trim_matches('-').to_string()
}