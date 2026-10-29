use std::{
    cmp::Ordering,
    path::Path,
    sync::OnceLock,
    time::{SystemTime, UNIX_EPOCH},
};

use percent_encoding::{utf8_percent_encode, AsciiSet, NON_ALPHANUMERIC};
use time::{format_description::FormatItem, macros::format_description, OffsetDateTime, UtcOffset};

static LOCAL_OFFSET: OnceLock<UtcOffset> = OnceLock::new();

/// 在进程启动、线程创建之前调用一次，拿到本机时区偏移。
pub fn init_local_offset() {
    let _ = LOCAL_OFFSET.set(UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC));
}

/// 路径片段里需要转义的字符集合（保留 `-._~`）。
const PATH_SEGMENT: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'.')
    .remove(b'_')
    .remove(b'~');

const DATE_FMT: &[FormatItem<'static>] =
    format_description!("[year]-[month]-[day] [hour]:[minute]");

pub fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

/// 把相对路径编码成可以直接放进 URL 的形式（逐段编码，保留 `/`）。
pub fn enc_path(path: &str) -> String {
    path.split('/')
        .map(|seg| utf8_percent_encode(seg, PATH_SEGMENT).to_string())
        .collect::<Vec<_>>()
        .join("/")
}

pub fn page(title: &str, body: &str, extra_head: &str) -> String {
    format!(
        r#"<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title}</title>
<link rel="stylesheet" href="/__asset/app.css">
<script>
(function () {{
  var t = localStorage.getItem('docserver-theme');
  if (t) document.documentElement.setAttribute('data-theme', t);
}})();
</script>
{extra_head}
</head>
<body>
<header class="topbar">
  <a class="brand" href="/">docserver</a>
  <button id="theme-toggle" class="icon-btn" title="切换深浅色" aria-label="切换深浅色">◐</button>
</header>
{body}
<script src="/__asset/app.js"></script>
</body>
</html>"#,
        title = esc(title),
        body = body,
        extra_head = extra_head,
    )
}

pub fn human_size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

pub fn mtime(time: SystemTime) -> String {
    let secs = time
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let offset = LOCAL_OFFSET.get().copied().unwrap_or(UtcOffset::UTC);
    match OffsetDateTime::from_unix_timestamp(secs) {
        Ok(t) => t
            .to_offset(offset)
            .format(DATE_FMT)
            .unwrap_or_else(|_| "—".to_string()),
        Err(_) => "—".to_string(),
    }
}

/// 让 `file2` 排在 `file10` 前面的比较方式。
pub fn natural_cmp(a: &str, b: &str) -> Ordering {
    let (mut ai, mut bi) = (a.chars().peekable(), b.chars().peekable());
    loop {
        match (ai.peek().copied(), bi.peek().copied()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(x), Some(y)) => {
                if x.is_ascii_digit() && y.is_ascii_digit() {
                    let mut xs = String::new();
                    while let Some(c) = ai.peek().copied().filter(|c| c.is_ascii_digit()) {
                        xs.push(c);
                        ai.next();
                    }
                    let mut ys = String::new();
                    while let Some(c) = bi.peek().copied().filter(|c| c.is_ascii_digit()) {
                        ys.push(c);
                        bi.next();
                    }
                    let xt = xs.trim_start_matches('0');
                    let yt = ys.trim_start_matches('0');
                    let ord = xt.len().cmp(&yt.len()).then_with(|| xt.cmp(yt));
                    if ord != Ordering::Equal {
                        return ord;
                    }
                } else {
                    let xl = x.to_lowercase().to_string();
                    let yl = y.to_lowercase().to_string();
                    let ord = xl.cmp(&yl);
                    if ord != Ordering::Equal {
                        return ord;
                    }
                    ai.next();
                    bi.next();
                }
            }
        }
    }
}

pub fn is_markdown(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("md" | "markdown" | "mdown" | "mkd")
    )
}

pub fn is_image(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase()),
        Some(ref e) if matches!(e.as_str(), "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" | "bmp" | "ico" | "avif")
    )
}

pub fn is_html(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("html" | "htm")
    )
}

/// 绝对 URL（协议、mailto、data）或站内绝对路径不需要改写。
pub fn is_external(url: &str) -> bool {
    url.starts_with('#')
        || url.starts_with('/')
        || url.starts_with("//")
        || url.starts_with("data:")
        || url.starts_with("mailto:")
        || url.starts_with("tel:")
        || url.split_once(':').map_or(false, |(scheme, _)| {
            !scheme.is_empty()
                && scheme
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '-' || c == '.')
        })
}

/// 把相对 URL 拼到当前目录上，并消掉 `./` 与 `../`。
pub fn join_rel(dir: &str, url: &str) -> String {
    let (path, suffix) = match url.find(['#', '?']) {
        Some(i) => (&url[..i], &url[i..]),
        None => (url, ""),
    };
    let mut parts: Vec<&str> = Vec::new();
    if !path.starts_with('/') {
        for seg in dir.split('/') {
            if !seg.is_empty() {
                parts.push(seg);
            }
        }
    }
    for seg in path.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            s => parts.push(s),
        }
    }
    format!("{}{}", parts.join("/"), suffix)
}