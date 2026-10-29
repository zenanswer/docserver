# docserver

本地文档服务器：一个二进制，把任意文件夹变成可以浏览的文档站。

- **目录浏览**：面包屑、文件夹优先、自然排序（`file2` 在 `file10` 前）、按文件名过滤
- **Markdown**：GFM 渲染（表格、任务列表、脚注、删除线），相对图片、相对 `.md` 链接直接可用，右侧生成目录
- **图片**：查看器支持滚轮缩放（5%–1200%）、拖拽平移、双击切换适应/1:1、左右方向键翻页
- **HTML**：原样返回，页面里的相对图片和样式照常工作
- **深浅色**：默认跟随系统，右上角可手动切换（记住选择）

## 编译与安装

```bash
cd ~/dev/docserver
cargo build --release
install -m755 target/x86_64-unknown-linux-musl/release/docserver ~/.local/bin/docserver
```

`.cargo/config.toml` 把默认目标设成 `x86_64-unknown-linux-musl`，所以 `cargo build --release`
直接产出**静态链接**的二进制（`ldd` 显示 `not a dynamic executable`），不依赖 glibc，
也不需要目标机器装 musl。想编动态链接版：

```bash
cargo build --release --target x86_64-unknown-linux-gnu
```

发布版由 `.github/workflows/release.yml` 在打 `v*` tag 时自动构建，
产出 `docserver-x86_64-unknown-linux-musl` 和 `docserver-aarch64-unknown-linux-musl`。

## 兼容性

| 项 | 范围 |
|---|---|
| libc | 不需要。静态链接 musl，不读目标机的 glibc 版本 |
| 内核 | Linux ≥ 3.2（Rust std 下限；2014 年后的发行版都满足） |
| 架构 | x86_64 与 aarch64（各自独立二进制，不能混用） |
| 运行时 | 单文件，无其他依赖

## 用法

```bash
docserver [DIR] [-h ADDR] [-p PORT] [-o] [--no-hide]
```

| 参数 | 默认 | 说明 |
|---|---|---|
| `DIR` | `.` | 要服务的文件夹 |
| `-h, --host` | `0.0.0.0` | 监听地址 |
| `-p, --port` | `8080` | 端口，`0` 表示系统分配并打印真实端口 |
| `-o, --open` | 关 | 启动后打开浏览器 |
| `--no-hide` | 关 | 显示以 `.` 开头的文件 |
| `--help` | | 帮助（`-h` 已分配给 host） |

```bash
docserver .                       # 当前目录，0.0.0.0:8080
docserver ~/notes -p 0            # 系统分配端口
docserver ~/notes -h 127.0.0.1 -p 9000
```

启动输出：

```
docserver 0.1.0
  根目录  ~/notes
  端口    44615
  本机    http://127.0.0.1:44615
  局域网  http://192.168.1.20:44615
```

## 路由语义

| 路径 | 行为 |
|---|---|
| `/a/b/` | 目录列表；目录里有 `index.html` 时顶部给一个「打开页面」按钮 |
| `/a/x.md` | 渲染成页面，内嵌图片走原始文件，链接到其它 `.md` 继续渲染 |
| `/a/pic.png` | 原始字节（`Content-Type` 正确），HTML 里可以直接引用 |
| `/__view/a/pic.png` | 图片查看器 |
| `/a/x.html` | 原样返回，页面自己的相对资源正常加载 |
| `/__raw/<path>` | 强制原始字节（任何类型），Markdown 页脚「查看原文」用它 |
| `/__asset/<name>` | 内置 CSS/JS（编译进二进制） |

## 查看器快捷键

| 操作 | 效果 |
|---|---|
| 滚轮 | 以光标为中心缩放 |
| 拖拽 | 平移 |
| 双击 | 适应窗口 ⇄ 原始大小 |
| `+` / `-` | 放大 / 缩小 |
| `0` / `1` | 适应窗口 / 原始大小 |
| `←` / `→` | 同目录上一张 / 下一张 |

## 安全

- 所有路径 `canonicalize` 后必须落在根目录内，否则 403
- 指向根目录之外的符号链接不列出、也访问不到
- 只读：没有上传、删除、编辑接口

## 已知限制

- 不实现 HTTP Range 请求，超大视频不会边下边播（图片和文档不受影响）
- 代码块不做语法高亮（只有配色和复制按钮）
- 一次只服务一个根目录；要换目录就换 `DIR`
- Markdown 的脚注、任务列表都支持，但不支持 mermaid 图表