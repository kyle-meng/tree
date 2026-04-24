<div align="center">

# 🌳 Tree

**An extremely lightweight, blazing-fast directory scanner and direct-link indexing service written in Rust.**

[![Release](https://img.shields.io/github/v/release/kyle-meng/tree?style=flat-square&color=2ea44f)](https://github.com/kyle-meng/tree/releases)
[![Build Status](https://img.shields.io/github/actions/workflow/status/kyle-meng/tree/release.yml?style=flat-square)](https://github.com/kyle-meng/tree/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg?style=flat-square)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg?style=flat-square)](https://www.rust-lang.org)

[Report Bug](https://github.com/kyle-meng/tree/issues) · [Request Feature](https://github.com/kyle-meng/tree/issues)

</div>

---

## 📖 简介

`tree` 是使用 Rust 编写的极其轻量的“目录扫描器”或“直链索引服务”。它抛弃了臃肿的框架，采用流式渲染和异步运行时，专注于提供极致的轻量化体验。无论是在服务器、软路由还是低配设备上，它都能以极低的资源占用稳定运行。

## ✨ 核心特性

- ⚡️ **极致轻量与高性能**
  - **内存友好**: 采用流式 HTML 生成技术（DFS 遍历），无需在内存中构建完整目录树。启动占用仅 **1.4MiB**，运行期间内存仅约 **2-3.0MiB**。
  - **异步高并发**: 基于 `Axum` + `Tokio` 异步运行时，精简依赖包，响应极快。
  - **极限压缩**: 针对 Release 模式进行了极致优化（LTO, Strip, `opt-level="z"`），部署零负担。
- 🔍 **智能搜索与聚合 API**
  - **实时搜索**: 集成轻量级前端搜索框，支持实时过滤文件和文件夹（完美支持中文）。
  - **全量索引 API**: 通过 `/api/links` 接口，一键获取所有文件的 JSON 索引，包含路径、URL、大小和修改时间。
- 🛡️ **安全与便捷**
  - **登录保护**: 支持通过命令行参数快速开启 HTTP Basic Auth 认证。
  - **URL 智能编码**: 自动处理路径中的空格、中文字符及特殊符号，生成标准百分号编码的 URL 链接。
- 🎨 **现代化交互体验**
  - **自适应暗黑模式**: 界面支持跟随操作系统的明亮/暗黑主题自动切换。
  - **精美视图与快捷操作**: 实时显示文件总数和总大小；支持一键复制文件直链（🔗）和生成简洁二维码（📱）。
  - **智能文件图标**: 自动根据文件扩展名展示不同的专属图标（视频、音频、图片、文档、代码等）。
- 🔗 **短链与重定向**
  - **智能重定向**: 提供 `/s/:key` 接口，通过哈希 ID 自动定位并重定向至原始文件，实现无状态的短链接服务。

---

## 🚀 快速开始

### 一键安装脚本 (推荐 - macOS / Linux)

使用一行命令自动检测系统架构并下载最新二进制文件：

```bash
curl -fsSL https://raw.githubusercontent.com/kyle-meng/tree/main/install.sh | bash
```

> **注**：该脚本会自动从 GitHub Releases 下载最新的预编译二进制文件并移动到 `/usr/local/bin`。

<details>
<summary><b>🛠 点击查看: 手动安装与脚本逻辑</b></summary>

如果您想手动执行或在 Windows (Git Bash) 下使用，也可以参考脚本逻辑：

```bash
#!/bin/bash
set -e

OS="$(uname -s | tr 'A-Z' 'a-z')"
ARCH="$(uname -m)"
VERSION="v0.2.1"
BASE_URL="https://github.com/kyle-meng/tree/releases/download/$VERSION"

case "$OS" in
  linux)
    if [ "$ARCH" = "x86_64" ]; then
      BINARY="tree-x86_64-unknown-linux-musl"
    elif [ "$ARCH" = "aarch64" ]; then
      BINARY="tree-aarch64-unknown-linux-musl"
    fi
    ;;
  darwin)
    if [ "$ARCH" = "arm64" ]; then
      BINARY="tree-aarch64-apple-darwin"
    fi
    ;;
  msys*|mingw*|cygwin*)
    BINARY="tree-x86_64-pc-windows-gnu.exe"
    ;;
esac

if [ -z "$BINARY" ]; then
  echo "Unsupported OS or Architecture: $OS / $ARCH"
  exit 1
fi

INSTALL_DIR="/usr/local/bin"
if [ ! -d "$INSTALL_DIR" ]; then
  INSTALL_DIR="/usr/bin"
fi

echo "Downloading $BINARY..."
curl -L "$BASE_URL/$BINARY" -o tree
chmod +x tree

if [ "$(id -u)" -eq 0 ]; then
  mv tree "$INSTALL_DIR/"
else
  if command -v sudo >/dev/null 2>&1; then
    sudo mv tree "$INSTALL_DIR/"
  else
    echo "Error: Need root privileges to install to $INSTALL_DIR. Please run as root or install 'sudo'."
    exit 1
  fi
fi

echo "Installation complete!"
```
</details>

---

## 💻 运行与使用

默认在当前目录启动服务，监听 `0.0.0.0:3000`，文件服务前缀为 `/files`。

### 基础运行
```bash
# 提供 /path/to/files 目录的服务
tree --dir /path/to/files
```

### 开启安全认证
```bash
# 开启登录保护 (默认用户: admin, 密码: 123456)
tree --dir /path/to/files --pass 123456
```

### 启动日志示例

启动成功后会在终端输出二维码及启动信息：

```text
Tree service starting...
Serving directory: "/path/to/files"
Prefix: /files
Access URL: http://0.0.0.0:3000
Protection: Enabled (User: admin)

Scan to access:
                             
   ▄▄▄▄▄   ▄   ▄     ▄▄▄▄▄  
   ▄▄▄   ▄▄  ▄   ▄   ▄▄▄    
  ▄▄▄▄▄▄▄ ▄  ▄▄  ▄  ▄▄▄▄▄▄▄ 
  ▄▄  ▄ ▄ ▄ ▄ ▄▄▄  ▄▄     ▄ 
   ▄▄▄▄ ▄▄▄▄▄▄▄▄  ▄  ▄▄  ▄▄ 
  ▄▄  ▄▄▄▄  ▄▄   ▄ ▄ ▄  ▄▄  
  ▄▄▄▄  ▄ ▄ ▄  ▄ ▄▄▄▄  ▄▄▄▄ 
  ▄▄    ▄▄ ▄▄   ▄▄ ▄▄▄    ▄ 
   ▄▄▄▄▄  ▄▄▄▄      ▄  ▄▄ ▄ 
          ▄ ▄▄▄▄ ▄  ▄▄  ▄ ▄ 
    ▄▄▄     ▄ ▄ ▄▄▄▄ ▄▄ ▄ ▄ 
  ▄▄▄▄▄▄▄ ▄▄▄ ▄ ▄ ▄▄▄▄   ▄▄ 
              
```

---

## ⚙️ 命令行参数

详尽的命令行选项让您可以灵活地定制服务：

| 参数 | 缩写 | 默认值 | 描述 |
| :--- | :---: | :--- | :--- |
| `--dir` | `-d` | `.` | 要提供服务的本地目录路径 |
| `--port` | `-p` | `3000` | 服务监听的端口号 |
| `--host` | | `0.0.0.0` | 绑定的 IP 地址 |
| `--prefix` | | `/files` | 文件服务在 URL 中的路由前缀 |
| `--pass` | | _无_ | 开启登录保护的密码，不设置则表示公开访问 |
| `--user` | | `admin` | 配合 `--pass` 使用的登录用户名 |

---

## 📡 API 接口参考

### `GET /api/links`

获取当前目录下所有文件的扁平化详细索引。对于批量下载或与其他系统集成非常有用。

**响应示例 (JSON):**

```json
[
  {
    "name": "电影/泰坦尼克号.mp4",
    "url": "http://localhost:3000/files/%E7%94%B5%E5%BD%B1/%E6%B3%B0%E5%9D%A6%E5%B0%BC%E5%85%8B%E5%8F%B7.mp4",
    "size": 2147483648,
    "modified": "2024-04-23 14:20:05"
  },
  {
    "name": "docs/说明文档.pdf",
    "url": "http://localhost:3000/files/docs/%E8%AF%B4%E6%98%8E%E6%96%87%E6%A1%A3.pdf",
    "size": 1048576,
    "modified": "2024-04-22 09:15:00"
  }
]
```

### 访问路径指南

- **可视化 Web 界面**: `http://localhost:3000/`
- **JSON 数据接口**: `http://localhost:3000/api/links`
- **文件直链访问**: `http://localhost:3000/files/path/to/file`

---

## 🛠️ 源码构建与优化

### 依赖环境

- [Rust](https://www.rust-lang.org/tools/install) >= 1.75.0

### 标准编译

```bash
cargo build --release
```

### musl 静态编译 (适用于 Alpine 或无 glibc 环境)

为了在轻量级容器或嵌入式设备中获得最大兼容性，可以使用 `musl` 构建完全静态链接的二进制文件：

```bash
rustup target add x86_64-unknown-linux-musl
cargo build --release --target x86_64-unknown-linux-musl
```

---

<div align="center">
  <p>Made with ❤️ by the open source community.</p>
  <p>
    <a href="https://github.com/kyle-meng/tree/stargazers">Star the repo</a> •
    <a href="https://github.com/kyle-meng/tree/issues">Report bugs</a>
  </p>
</div>
