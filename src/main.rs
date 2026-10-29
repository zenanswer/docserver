mod assets;
mod browse;
mod markdown;
mod serve;
mod ui;
mod viewer;

use std::{
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{bail, Context, Result};
use axum::{routing::get, Router};
use clap::Parser;

/// 全局状态，所有 handler 共享。
#[derive(Clone)]
pub struct App {
    /// 已经 canonicalize 过的根目录
    pub root: Arc<PathBuf>,
    /// 是否显示以 `.` 开头的文件
    pub show_hidden: bool,
}

#[derive(Parser, Debug)]
#[command(
    name = "docserver",
    version,
    about = "本地文档服务器：目录浏览 / Markdown 渲染 / 图片查看 / HTML 原样服务",
    disable_help_flag = true
)]
struct Args {
    /// 要服务的文件夹
    #[arg(default_value = ".")]
    dir: PathBuf,

    /// 监听地址
    #[arg(short = 'h', long = "host", default_value = "0.0.0.0", value_name = "ADDR")]
    host: String,

    /// 端口，0 表示由系统分配
    #[arg(short = 'p', long = "port", default_value_t = 8080, value_name = "PORT")]
    port: u16,

    /// 启动后打开浏览器
    #[arg(short = 'o', long = "open")]
    open: bool,

    /// 显示隐藏文件
    #[arg(long = "no-hide")]
    show_hidden: bool,

    /// 打印帮助
    #[arg(long = "help", action = clap::ArgAction::Help)]
    help: Option<bool>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    ui::init_local_offset();

    let root = std::fs::canonicalize(&args.dir)
        .with_context(|| format!("打不开目录：{}", args.dir.display()))?;
    if !root.is_dir() {
        bail!("不是目录：{}", root.display());
    }

    let app = App {
        root: Arc::new(root.clone()),
        show_hidden: args.show_hidden,
    };

    let addr: SocketAddr = format!("{}:{}", args.host, args.port)
        .parse()
        .with_context(|| format!("监听地址不合法：{}:{}", args.host, args.port))?;

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    runtime.block_on(async move {
        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .with_context(|| format!("端口被占用或无权限：{addr}"))?;
        let local = listener.local_addr()?;

        let router = Router::new()
            .route("/__asset/{*name}", get(assets::handler))
            .route("/__raw/{*path}", get(serve::raw_handler))
            .route("/__view/{*path}", get(viewer::handler))
            .route("/", get(serve::root_handler))
            .route("/{*path}", get(serve::handler))
            .with_state(app);

        print_banner(&root, local);

        if args.open {
            let url = format!("http://127.0.0.1:{}", local.port());
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(400));
                let _ = std::process::Command::new("xdg-open").arg(url).spawn();
            });
        }

        axum::serve(listener, router).await?;
        Ok::<(), anyhow::Error>(())
    })?;

    Ok(())
}

fn print_banner(root: &Path, addr: SocketAddr) {
    println!("docserver {}", env!("CARGO_PKG_VERSION"));
    println!("  根目录  {}", root.display());
    println!("  端口    {}", addr.port());

    if addr.ip().is_unspecified() {
        println!("  本机    http://127.0.0.1:{}", addr.port());
        if let Ok(ip) = local_ip_address::local_ip() {
            if !ip.is_loopback() && !ip.is_unspecified() {
                println!("  局域网  http://{}:{}", ip, addr.port());
            }
        }
    } else {
        println!("  地址    http://{}:{}", addr.ip(), addr.port());
    }
    println!();
}