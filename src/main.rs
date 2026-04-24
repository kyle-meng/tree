use axum::{
    extract::{Host, Path as AxumPath, Request, State},
    middleware::{self, Next},
    response::{Html, IntoResponse, Redirect, Response},
    routing::get,
    Json, Router,
};
use base64::{engine::general_purpose::STANDARD, engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chrono::{DateTime, Local};
use clap::Parser;
use local_ip_address::local_ip;
use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};
use std::{
    fmt::Write,
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::Arc,
};
use tower_http::services::ServeDir;
use walkdir::WalkDir;

const FRAGMENT: &AsciiSet = &CONTROLS.add(b' ').add(b'"').add(b'<').add(b'>').add(b'`');
const PATH_ENCODE_SET: &AsciiSet = &FRAGMENT.add(b'#').add(b'?').add(b'{').add(b'}').add(b'[').add(b']').add(b'^').add(b'|').add(b'%');

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = ".")]
    dir: PathBuf,

    #[arg(long, default_value = "0.0.0.0")]
    host: String,

    #[arg(short, long, default_value_t = 3000)]
    port: u16,

    #[arg(long, default_value = "/files")]
    prefix: String,

    #[arg(long, default_value = "admin")]
    user: String,

    #[arg(long)]
    pass: Option<String>,
}

struct AppState {
    root_dir: PathBuf,
    prefix: String,
    username: String,
    password: Option<String>,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let mut prefix = args.prefix.clone();
    if !prefix.starts_with('/') { prefix.insert(0, '/'); }
    if prefix.ends_with('/') && prefix.len() > 1 { prefix.pop(); }

    let root_dir = std::fs::canonicalize(&args.dir).unwrap_or_else(|_| args.dir.clone());

    let state = Arc::new(AppState {
        root_dir: root_dir.clone(),
        prefix: prefix.clone(),
        username: args.user.clone(),
        password: args.pass.clone(),
    });

    let mut app = Router::new()
        .route("/", get(handle_html_tree))
        .route("/api/links", get(handle_api_links))
        .route("/s/:key", get(handle_short_link))
        .nest_service(&prefix, ServeDir::new(&root_dir));

    if state.password.is_some() {
        app = app.layer(middleware::from_fn_with_state(state.clone(), auth_middleware));
    }

    let app = app.with_state(state.clone());

    let addr: SocketAddr = format!("{}:{}", args.host, args.port).parse().expect("Invalid host or port");

    println!("Tree service starting...");
    println!("Serving directory: {:?}", root_dir);
    println!("Prefix: {}", prefix);
    
    let display_host = if args.host == "0.0.0.0" {
        local_ip().map(|ip| ip.to_string()).unwrap_or_else(|_| "127.0.0.1".into())
    } else {
        args.host.clone()
    };
    
    let access_url = format!("http://{}:{}", display_host, args.port);
    println!("Access URL: {}", access_url);
    if state.password.is_some() {
        println!("Protection: Enabled (User: {})", state.username);
    }

    println!("\nScan to access:");
    qr2term::print_qr(&access_url).ok();
    println!("");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn auth_middleware(State(state): State<Arc<AppState>>, req: Request, next: Next) -> Response {
    if let Some(ref expected_pass) = state.password {
        let auth_header = req.headers().get(axum::http::header::AUTHORIZATION).and_then(|h| h.to_str().ok());
        let expected_auth = format!("{}:{}", state.username, expected_pass);
        let expected_header = format!("Basic {}", STANDARD.encode(expected_auth));
        if auth_header != Some(&expected_header) {
            return (axum::http::StatusCode::UNAUTHORIZED, [(axum::http::header::WWW_AUTHENTICATE, "Basic realm=\"Tree Service\"")], "Unauthorized").into_response();
        }
    }
    next.run(req).await
}

fn hash_path(path: &str) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for b in path.as_bytes() {
        hash ^= *b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    URL_SAFE_NO_PAD.encode(hash.to_le_bytes())
}

async fn handle_short_link(AxumPath(key): AxumPath<String>, State(state): State<Arc<AppState>>) -> impl IntoResponse {
    for entry in WalkDir::new(&state.root_dir).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() {
            if let Ok(rel_path) = entry.path().strip_prefix(&state.root_dir) {
                let path_str = rel_path.to_string_lossy();
                if hash_path(&path_str) == key {
                    let encoded_path = encode_path(&path_str);
                    return Redirect::temporary(&format!("{}/{}", state.prefix, encoded_path)).into_response();
                }
            }
        }
    }
    axum::http::StatusCode::NOT_FOUND.into_response()
}

fn encode_path(path: &str) -> String {
    path.split(|c| c == '/' || c == '\\').map(|segment| utf8_percent_encode(segment, PATH_ENCODE_SET).to_string()).collect::<Vec<_>>().join("/")
}

#[derive(serde::Serialize)]
struct FileLink {
    name: String,
    url: String,
    size: u64,
    modified: String,
}

async fn handle_api_links(Host(host): Host, State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let mut links = Vec::new();
    for entry in WalkDir::new(&state.root_dir).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() {
            if let Ok(rel_path) = entry.path().strip_prefix(&state.root_dir) {
                let path_str = rel_path.to_string_lossy().to_string();
                let encoded_rel_path = encode_path(&path_str);
                let url = format!("http://{}{}/{}", host, state.prefix, encoded_rel_path);
                let metadata = entry.metadata().ok();
                let size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);
                let modified = metadata.as_ref().and_then(|m| m.modified().ok()).map(|t| { let dt: DateTime<Local> = t.into(); dt.format("%Y-%m-%d %H:%M:%S").to_string() }).unwrap_or_default();
                links.push(FileLink { name: path_str, url, size, modified });
            }
        }
    }
    Json(links)
}

fn get_file_icon(path: &Path) -> &'static str {
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
    match ext.as_str() {
        "mp4" | "mkv" | "avi" | "mov" | "flv" | "wmv" | "webm" | "mpg" | "mpeg" => "🎬",
        "mp3" | "wav" | "flac" | "ogg" | "m4a" | "aac" | "wma" => "🎵",
        "jpg" | "jpeg" | "png" | "gif" | "webp" | "svg" | "bmp" | "ico" => "🖼️",
        "pdf" | "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" => "📄",
        "txt" | "md" | "json" | "xml" | "yml" | "yaml" | "csv" | "conf" | "sh" | "rs" | "py" | "js" | "html" | "css" => "📝",
        "zip" | "rar" | "7z" | "tar" | "gz" | "bz2" | "xz" => "📦",
        "exe" | "msi" | "deb" | "rpm" | "apk" => "💿",
        _ => "📄",
    }
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024; const MB: u64 = KB * 1024; const GB: u64 = MB * 1024;
    if bytes >= GB { format!("{:.2} GB", bytes as f64 / GB as f64) } 
    else if bytes >= MB { format!("{:.2} MB", bytes as f64 / MB as f64) } 
    else if bytes >= KB { format!("{:.2} KB", bytes as f64 / KB as f64) } 
    else { format!("{} B", bytes) }
}

struct TreeStats { count: u64, size: u64 }

async fn handle_html_tree(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let mut tree_html = String::with_capacity(32768);
    let mut stats = TreeStats { count: 0, size: 0 };
    render_html_recursive(&state.root_dir, &state.root_dir, &state.prefix, &mut tree_html, &mut stats);
    let mut html = String::with_capacity(tree_html.len() + 8192);
    html.push_str(r##"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Tree Index</title>
    <style>
        :root { --bg: #f9f9f9; --container-bg: #ffffff; --text: #333; --text-muted: #999; --border: #eee; --accent: #3498db; --hover: #f0f0f0; --item-border: #f0f0f0; }
        @media (prefers-color-scheme: dark) {
            :root { --bg: #1a1a1a; --container-bg: #2d2d2d; --text: #e0e0e0; --text-muted: #888; --border: #444; --accent: #3498db; --hover: #3d3d3d; --item-border: #3d3d3d; }
        }
        body { font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif; padding: 20px; line-height: 1.6; color: var(--text); max-width: 1200px; margin: 0 auto; background-color: var(--bg); transition: background 0.3s; }
        h1 { border-bottom: 2px solid var(--border); padding-bottom: 10px; color: var(--accent); }
        ul { list-style-type: none; padding-left: 24px; margin: 0; }
        li { margin: 4px 0; }
        details { margin: 2px 0; }
        summary { cursor: pointer; color: var(--accent); font-weight: 600; outline: none; list-style: none; display: flex; align-items: center; }
        summary::-webkit-details-marker { display: none; }
        summary::before { content: "📁 "; display: inline-block; width: 1.5em; flex-shrink: 0; }
        details[open] > summary::before { content: "📂 "; }
        a { color: var(--text); text-decoration: none; border-radius: 3px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
        a:hover { background-color: var(--hover); color: #e67e22; }
        .file { display: flex; align-items: center; border-bottom: 1px solid var(--item-border); padding: 4px 0; }
        .file:hover { background-color: var(--hover); }
        .file-icon { display: inline-block; width: 1.5em; flex-shrink: 0; }
        .file-name { flex: 1; min-width: 0; display: flex; align-items: center; }
        .file-info { font-size: 0.85em; color: var(--text-muted); margin-left: 15px; white-space: nowrap; display: flex; gap: 15px; align-items: center; }
        .file-size { width: 80px; text-align: right; }
        .file-date { width: 140px; text-align: right; }
        .action-btn { cursor: pointer; opacity: 0.3; transition: opacity 0.3s; font-size: 0.9em; flex-shrink: 0; user-select: none; padding: 0 4px; }
        .action-btn:hover { opacity: 1; }
        .container { background: var(--container-bg); padding: 30px; border-radius: 8px; box-shadow: 0 2px 10px rgba(0,0,0,0.2); }
        .stats { margin-top: -10px; margin-bottom: 20px; font-size: 0.9em; color: var(--text-muted); border-bottom: 1px solid var(--border); padding-bottom: 10px; }
        .search-box { margin-bottom: 20px; position: sticky; top: 0; background: var(--container-bg); padding: 10px 0; z-index: 100; }
        #search { width: 100%; padding: 12px; border: 2px solid var(--border); border-radius: 6px; font-size: 16px; transition: border-color 0.3s; background: var(--bg); color: var(--text); }
        #search:focus { border-color: var(--accent); outline: none; }
        #qr-modal { display: none; position: fixed; top: 0; left: 0; width: 100%; height: 100%; background: rgba(0,0,0,0.7); z-index: 1000; justify-content: center; align-items: center; backdrop-filter: blur(4px); }
        .qr-content { background: #fff; padding: 30px; border-radius: 12px; text-align: center; position: relative; box-shadow: 0 10px 25px rgba(0,0,0,0.3); max-width: 90%; color: #333; }
        .qr-close { position: absolute; top: 10px; right: 15px; cursor: pointer; font-size: 24px; color: #999; }
        #qr-code img { margin: 0 auto; }
        #qr-url { margin-top: 15px; font-size: 0.8em; color: #666; word-break: break-all; max-width: 320px; }
        .toast { position: fixed; bottom: 20px; left: 50%; transform: translateX(-50%); background: var(--accent); color: white; padding: 8px 16px; border-radius: 20px; font-size: 0.9em; opacity: 0; transition: opacity 0.3s; z-index: 2000; }
        footer { margin-top: 40px; font-size: 0.8em; color: var(--text_muted); text-align: center; }
        code { background: var(--hover); padding: 2px 4px; border-radius: 3px; font-family: monospace; }
        @media (max-width: 600px) {
            .file-date { display: none; }
            .file-info { gap: 8px; }
            .container { padding: 15px; }
        }
    </style>
    <script src="https://cdn.jsdelivr.net/npm/qrcodejs@1.0.0/qrcode.min.js"></script>
</head>
<body>
    <div class="container">
        <h1>Tree Index</h1>
        <div class="stats">📊 Total: <strong>"##);
    let _ = write!(html, "{}</strong> files, <strong>{}</strong>", stats.count, format_size(stats.size));
    html.push_str(r##"        </div>
        <div class="search-box">
            <input type="text" id="search" placeholder="Search files and folders..." autocomplete="off">
        </div>
        <div id="tree-root">"##);
    html.push_str(&tree_html);
    html.push_str(r##"        </div>
    </div>
    <div id="qr-modal" onclick="closeQR()"><div class="qr-content" onclick="event.stopPropagation()"><span class="qr-close" onclick="closeQR()">&times;</span><div id="qr-code"></div><div id="qr-url"></div></div></div>
    <div id="toast" class="toast">URL Copied!</div>
    <footer><p>Generated by <code>tree</code> - Lightweight Directory Indexer</p></footer>
    <script>
        const searchInput = document.getElementById('search');
        const treeRoot = document.getElementById('tree-root');
        function showQR(shortId) {
            const modal = document.getElementById('qr-modal'); const container = document.getElementById('qr-code'); const urlText = document.getElementById('qr-url');
            const shortUrl = window.location.origin + '/s/' + shortId;
            container.innerHTML = ''; urlText.textContent = shortUrl;
            new QRCode(container, { text: shortUrl, width: 320, height: 320, colorDark : "#2c3e50", colorLight : "#ffffff", correctLevel : QRCode.CorrectLevel.L });
            modal.style.display = 'flex';
        }
        function closeQR() { document.getElementById('qr-modal').style.display = 'none'; }
        function copyLink(url) {
            if (navigator.clipboard && window.isSecureContext) { navigator.clipboard.writeText(url).then(() => showToast()); } 
            else { const textArea = document.createElement("textarea"); textArea.value = url; textArea.style.position = "fixed"; textArea.style.left = "-999999px"; textArea.style.top = "-999999px"; document.body.appendChild(textArea); textArea.focus(); textArea.select(); try { document.execCommand('copy'); showToast(); } catch (err) {} document.body.removeChild(textArea); }
        }
        function showToast() { const toast = document.getElementById('toast'); toast.style.opacity = '1'; setTimeout(() => toast.style.opacity = '0', 2000); }
        searchInput.addEventListener('input', (e) => {
            const term = e.target.value.toLowerCase().trim();
            const allLis = treeRoot.querySelectorAll('li'); const allDetails = treeRoot.querySelectorAll('details');
            if (term === '') { allLis.forEach(li => li.style.display = ''); allDetails.forEach(details => { details.style.display = ''; details.open = true; }); return; }
            allLis.forEach(li => li.style.display = 'none'); allDetails.forEach(details => { details.style.display = 'none'; details.open = false; });
            const files = treeRoot.querySelectorAll('.file');
            files.forEach(file => {
                const nameLink = file.querySelector('a');
                if (nameLink && nameLink.textContent.toLowerCase().includes(term)) {
                    let li = file.parentElement; li.style.display = 'block'; let parent = li.parentElement;
                    while (parent && parent !== treeRoot) { if (parent.tagName === 'DETAILS') { parent.style.display = 'block'; parent.open = true; } else if (parent.tagName === 'LI' || parent.tagName === 'UL') { parent.style.display = 'block'; } parent = parent.parentElement; }
                }
            });
            allDetails.forEach(details => {
                const summary = details.querySelector('summary');
                if (summary && summary.textContent.toLowerCase().includes(term)) {
                    details.style.display = 'block'; details.open = true; details.querySelectorAll(':scope > ul > li').forEach(li => li.style.display = 'block');
                    let parent = details.parentElement; while (parent && parent !== treeRoot) { if (parent.tagName === 'DETAILS') { parent.style.display = 'block'; parent.open = true; } else if (parent.tagName === 'LI' || parent.tagName === 'UL') { parent.style.display = 'block'; } parent = parent.parentElement; }
                }
            });
        });
    </script>
</body>
</html>"##);
    Html(html)
}

fn render_html_recursive(root: &Path, current: &Path, prefix: &str, html: &mut String, stats: &mut TreeStats) {
    let name = current.file_name().and_then(|n| n.to_str()).unwrap_or("/");
    if current.is_dir() {
        let _ = write!(html, "<details open><summary>{}</summary><ul>", name);
        if let Ok(entries) = std::fs::read_dir(current) {
            let mut entries: Vec<_> = entries.filter_map(|e| e.ok()).collect();
            entries.sort_by(|a, b| { let a_is_dir = a.path().is_dir(); let b_is_dir = b.path().is_dir(); if a_is_dir != b_is_dir { b_is_dir.cmp(&a_is_dir) } else { a.file_name().cmp(&b.file_name()) } });
            for entry in entries { html.push_str("<li>"); render_html_recursive(root, &entry.path(), prefix, html, stats); html.push_str("</li>"); }
        }
        html.push_str("</ul></details>");
    } else if let Ok(rel_path) = current.strip_prefix(root) {
        let path_str = rel_path.to_string_lossy();
        let encoded_path = encode_path(&path_str);
        let icon = get_file_icon(current);
        let short_id = hash_path(&path_str);
        let metadata = current.metadata().ok();
        let size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);
        stats.count += 1; stats.size += size;
        let size_str = format_size(size);
        let date_str = metadata.as_ref().and_then(|m| m.modified().ok()).map(|t| { let dt: DateTime<Local> = t.into(); dt.format("%Y-%m-%d %H:%M").to_string() }).unwrap_or_else(|| "-".to_string());
        let full_url_js = format!("window.location.origin + '{}/{}'", prefix, encoded_path);
        let _ = write!(html, r##"<div class="file"><div class="file-name"><span class="file-icon">{}</span><a href="{}/{}">{}</a></div><div class="file-info"><span class="file-size">{}</span><span class="file-date">{}</span><span class="action-btn" title="Copy Direct Link" onclick="copyLink({})">🔗</span><span class="action-btn" title="Generate QR Code" onclick="showQR('{}')">📱</span></div></div>"##, icon, prefix, encoded_path, name, size_str, date_str, full_url_js, short_id);
    }
}
