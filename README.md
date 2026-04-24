# tree

tree 是使用 Rust 编写的极其轻量的“目录扫描器”或“直链索引服务”。

## 功能特性

- **一次性获取所有直链**: 通过 `/api/links` 接口获取所有文件的 JSON 索引，包含路径、URL、大小和修改时间。
- **静态文件服务器**: 极简的 HTML 目录树展示，支持文件夹折叠/展开。
- **简单登录保护**: 支持通过命令行参数开启 HTTP Basic Auth 认证。
- **智能搜索**: 集成轻量级前端搜索框，支持实时过滤文件和文件夹（支持中文）。
- **自动暗黑模式**: 界面支持跟随系统主题自动切换明亮/暗黑模式。
- **概览统计**: 页面顶部实时显示文件总数和总大小。
- **快捷操作**: 支持一键复制文件直链（🔗）和生成简洁二维码（📱）。
- **丰富图标**: 自动根据文件扩展名展示不同的图标（视频、音频、图片、文档、代码等）。
- **智能重定向**: 提供 `/s/:key` 接口，通过哈希 ID 自动定位并重定向至原始文件，实现无状态的短链接服务。
- **极致优化**:
    - **内存友好**: 采用流式 HTML 生成技术（DFS 遍历），无需在内存中构建完整目录树，启动占用1.4MiB，待机以及使用过程内存仅约 3.6 MiB。
    - **高性能**: 基于 Axum + Tokio 异步运行时，精简依赖包，响应极快。
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
# 默认运行 (无密码)
./target/release/tree --dir /path/to/files

# 开启登录保护 (用户: admin, 密码: 123456)
./target/release/tree --dir /path/to/files --pass 123456

Tree service starting...
Serving directory: "/path/to/files"
Prefix: /files
Access URL: http://[IP_ADDRESS]:3000
Protection: Enabled (User: admin)

Scan to access:
                             
   ▄▄▄▄▄   ▄   ▄     ▄▄▄▄▄   
              ▄  ▄           
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

### 访问

- **Web 界面**: 直接访问 `http://localhost:3000/`
- **JSON API**: 访问 `http://localhost:3000/api/links`
- **文件直链**: 访问 `http://localhost:3000/files/path/to/file`

## 命令行参数

| 参数 | 缩写 | 描述 | 默认值 |
| :--- | :--- | :--- | :--- |
| `--dir` | `-d` | 要服务的本地目录路径 | `.` |
| `--port` | `-p` | 服务监听端口 | `3000` |
| `--pass` | | 开启登录保护的密码 (不设置则不开启) | 无 |
| `--user` | | 登录用户名 | `admin` |
| `--prefix` | | 文件服务在 URL 中的前缀 | `/files` |
| `--host` | | 绑定地址 | `0.0.0.0` |

## 接口说明

### `GET /api/links`

返回一个包含所有文件详细信息的 JSON 对象数组。

**响应示例:**
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

## 编译优化 (musl 静态编译)

如需在无 glibc 的环境（如 Alpine Linux 或某些嵌入式设备）下运行，可进行静态编译：

```bash
rustup target add x86_64-unknown-linux-musl
cargo build --release --target x86_64-unknown-linux-musl
```
