# tree

tree 是使用 Rust 编写的极其轻量的“目录扫描器”或“直链索引服务”。

## 功能特性

- **一次性获取所有直链**: 通过 `/api/links` 接口获取所有文件的 JSON 索引，包含原始路径与编码后的 URL。
- **静态文件服务器**: 极简的 HTML 目录树展示，支持文件夹折叠/展开。
- **智能搜索**: 集成轻量级前端搜索框，支持实时过滤文件和文件夹（支持中文）。
- **丰富图标**: 自动根据文件扩展名展示不同的图标（视频、音频、图片、文档、代码等）。
- **扫码传输**: 点击文件旁的 📱 图标可即时生成二维码。采用哈希短链接技术，确保超长文件名也能生成易于扫描的简洁二维码。
- **智能重定向**: 提供 `/s/:key` 接口，通过哈希 ID 自动定位并重定向至原始文件，实现无状态的短链接服务。
- **极致优化**:
    - **内存友好**: 采用流式 HTML 生成技术（DFS 遍历），无需在内存中构建完整目录树。
    - **高性能**: 基于 Axum + Tokio 异步运行时，精简依赖包。
    - **二进制体积**: 针对 Release 模式进行了极限压缩（LTO, Strip, 优化等级 z）。
- **URL 安全**: 自动处理路径中的空格、中文字符及特殊符号，生成标准百分号编码的 URL。

## 快速开始

### 安装

确保你已经安装了 Rust 环境。

```bash
cargo build --release
```

### 运行

默认在当前目录启动服务，监听 `0.0.0.0:3000`，文件服务前缀为 `/files`。

```bash
./target/release/tree
```

或者使用参数：

```bash
./target/release/tree --dir /path/to/files --port 8080 --host 127.0.0.1 --prefix /download
```

## 命令行参数

| 参数 | 缩写 | 描述 | 默认值 |
| :--- | :--- | :--- | :--- |
| `--dir` | `-d` | 要服务的本地目录路径 | `.` |
| `--port` | `-p` | 服务监听端口 | `3000` |
| `--host` | | 绑定地址 | `0.0.0.0` |
| `--prefix` | | 文件服务在 URL 中的前缀 | `/files` |

## 接口说明

### `GET /api/links`

返回一个包含所有文件信息的对象数组。

**响应示例:**
```json
[
  {
    "name": "电影/泰坦尼克号.mp4",
    "url": "http://localhost:3000/files/%E7%94%B5%E5%BD%B1/%E6%B3%B0%E5%9D%A6%E5%B0%BC%E5%85%8B%E5%8F%B7.mp4"
  },
  {
    "name": "docs/manual [v1].pdf",
    "url": "http://localhost:3000/files/docs/manual%20%5Bv1%5D.pdf"
  }
]
```

## 编译优化 (musl 静态编译)

如需在无 Rust/glibc 环境下运行，可进行静态编译：

```bash
rustup target add x86_64-unknown-linux-musl
cargo build --release --target x86_64-unknown-linux-musl
```
