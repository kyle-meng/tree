# tree

tree 是使用 Rust 编写的极其轻量的“目录扫描器”或“直链索引服务”。

## 功能特性

- **一次性获取所有直链**: 通过 `/api/links` 接口以 JSON 格式输出所有文件的完整 URL。
- **静态文件服务器**: 以目录树形式展示文件列表，支持文件夹折叠/展开。
- **高性能**: 基于 Axum 和 Tokio 构建，异步非阻塞。
- **简单易用**: 支持命令行参数配置目录、主机、端口以及服务路径前缀。

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
./target/release/tree --dir /path/to/your/files --port 8080 --host 127.0.0.1 --prefix /download
```

### 访问

- **Web 界面 (HTML 树)**: 直接访问 `http://localhost:3000/`
- **JSON API (所有直链)**: 访问 `http://localhost:3000/api/links`
- **文件下载**: 访问 `http://localhost:3000/files/path/to/file` (或你设置的 `--prefix`)

## 接口说明

### `GET /api/links`

返回一个对象数组，每个对象包含原始文件路径和编码后的完整 URL。

**响应示例 (默认前缀):**
```json
[
  {
    "name": "image1.jpg",
    "url": "http://localhost:3000/files/image1.jpg"
  },
  {
    "name": "docs/说明文档.pdf",
    "url": "http://localhost:3000/files/docs/%E8%AF%B4%E6%98%8E%E6%96%87%E6%A1%A3.pdf"
  }
]
```
