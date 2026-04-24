use axum::{
    extract::{Host, State},
    response::{Html, IntoResponse},
    routing::get,
    Json, Router,
};
use clap::Parser;
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
}

struct AppState {
    root_dir: PathBuf,
    prefix: String,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let mut prefix = args.prefix.clone();
    if !prefix.starts_with('/') {
        prefix.insert(0, '/');
    }
    if prefix.ends_with('/') && prefix.len() > 1 {
        prefix.pop();
    }

    let root_dir = std::fs::canonicalize(&args.dir).unwrap_or_else(|_| args.dir.clone());

    let state = Arc::new(AppState {
        root_dir: root_dir.clone(),
        prefix: prefix.clone(),
    });

    let app = Router::new()
        .route("/", get(handle_html_tree))
        .route("/api/links", get(handle_api_links))
        .nest_service(&prefix, ServeDir::new(&root_dir))
        .with_state(state);

    let addr: SocketAddr = format!("{}:{}", args.host, args.port)
        .parse()
        .expect("Invalid host or port");

    println!("Tree service starting on http://{}", addr);
    println!("Serving directory: {:?}", root_dir);
    println!("Prefix: {}", prefix);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

fn encode_path(path: &str) -> String {
    path.split(|c| c == '/' || c == '\\')
        .map(|segment| utf8_percent_encode(segment, PATH_ENCODE_SET).to_string())
        .collect::<Vec<_>>()
        .join("/")
}

#[derive(serde::Serialize)]
struct FileLink {
    name: String,
    url: String,
}

async fn handle_api_links(
    Host(host): Host,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let mut links = Vec::new();

    for entry in WalkDir::new(&state.root_dir).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() {
            if let Ok(rel_path) = entry.path().strip_prefix(&state.root_dir) {
                let path_str = rel_path.to_string_lossy().to_string();
                let encoded_rel_path = encode_path(&path_str);
                let url = format!("http://{}{}/{}", host, state.prefix, encoded_rel_path);
                links.push(FileLink {
                    name: path_str,
                    url,
                });
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

async fn handle_html_tree(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let mut html = String::with_capacity(8192);
    html.push_str(r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Tree Index</title>
    <style>
        body { font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif; padding: 20px; line-height: 1.6; color: #333; max-width: 1200px; margin: 0 auto; background-color: #f9f9f9; }
        h1 { border-bottom: 2px solid #eee; padding-bottom: 10px; color: #2c3e50; }
        ul { list-style-type: none; padding-left: 24px; margin: 0; }
        li { margin: 4px 0; }
        details { margin: 2px 0; }
        summary { cursor: pointer; color: #3498db; font-weight: 600; outline: none; list-style: none; display: flex; align-items: center; }
        summary::-webkit-details-marker { display: none; }
        summary:hover { color: #2980b9; }
        summary::before { content: "📁 "; display: inline-block; width: 1.5em; flex-shrink: 0; }
        details[open] > summary::before { content: "📂 "; }
        a { color: #2c3e50; text-decoration: none; display: inline-block; padding: 2px 4px; border-radius: 3px; }
        a:hover { background-color: #e0e0e0; color: #e67e22; }
        .file { display: flex; align-items: center; }
        .file-icon { display: inline-block; width: 1.5em; flex-shrink: 0; }
        .container { background: white; padding: 30px; border-radius: 8px; box-shadow: 0 2px 10px rgba(0,0,0,0.05); }
        .search-box { margin-bottom: 20px; position: sticky; top: 0; background: white; padding: 10px 0; z-index: 100; }
        #search { width: 100%; padding: 12px; border: 2px solid #eee; border-radius: 6px; font-size: 16px; transition: border-color 0.3s; }
        #search:focus { border-color: #3498db; outline: none; }
        footer { margin-top: 40px; font-size: 0.8em; color: #95a5a6; text-align: center; }
        code { background: #f0f0f0; padding: 2px 4px; border-radius: 3px; font-family: monospace; }
    </style>
</head>
<body>
    <div class="container">
        <h1>Tree Index</h1>
        <div class="search-box">
            <input type="text" id="search" placeholder="Search files and folders..." autocomplete="off">
        </div>
        <div id="tree-root">"#);

    render_html_recursive(&state.root_dir, &state.root_dir, &state.prefix, &mut html);

    html.push_str(r#"        </div>
    </div>
    <footer>
        <p>Generated by <code>tree</code> - Lightweight Directory Indexer</p>
    </footer>
    <script>
        const searchInput = document.getElementById('search');
        const treeRoot = document.getElementById('tree-root');

        searchInput.addEventListener('input', (e) => {
            const term = e.target.value.toLowerCase().trim();
            const allLis = treeRoot.querySelectorAll('li');
            const allDetails = treeRoot.querySelectorAll('details');

            if (term === '') {
                allLis.forEach(li => li.style.display = '');
                allDetails.forEach(details => {
                    details.style.display = '';
                    details.open = true;
                });
                return;
            }

            allLis.forEach(li => li.style.display = 'none');
            allDetails.forEach(details => {
                details.style.display = 'none';
                details.open = false;
            });

            const files = treeRoot.querySelectorAll('.file');
            files.forEach(file => {
                if (file.textContent.toLowerCase().includes(term)) {
                    let li = file.parentElement;
                    li.style.display = 'block';
                    let parent = li.parentElement;
                    while (parent && parent !== treeRoot) {
                        if (parent.tagName === 'DETAILS') {
                            parent.style.display = 'block';
                            parent.open = true;
                        } else if (parent.tagName === 'LI' || parent.tagName === 'UL') {
                            parent.style.display = 'block';
                        }
                        parent = parent.parentElement;
                    }
                }
            });
            
            allDetails.forEach(details => {
                const summary = details.querySelector('summary');
                if (summary && summary.textContent.toLowerCase().includes(term)) {
                    details.style.display = 'block';
                    details.open = true;
                    details.querySelectorAll(':scope > ul > li').forEach(li => li.style.display = 'block');
                    let parent = details.parentElement;
                    while (parent && parent !== treeRoot) {
                        if (parent.tagName === 'DETAILS') {
                            parent.style.display = 'block';
                            parent.open = true;
                        } else if (parent.tagName === 'LI' || parent.tagName === 'UL') {
                            parent.style.display = 'block';
                        }
                        parent = parent.parentElement;
                    }
                }
            });
        });
    </script>
</body>
</html>"#);

    Html(html)
}

fn render_html_recursive(root: &Path, current: &Path, prefix: &str, html: &mut String) {
    let name = current
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("/");

    if current.is_dir() {
        let _ = write!(html, "<details open><summary>{}</summary><ul>", name);
        if let Ok(entries) = std::fs::read_dir(current) {
            let mut entries: Vec<_> = entries.filter_map(|e| e.ok()).collect();
            entries.sort_by(|a, b| {
                let a_is_dir = a.path().is_dir();
                let b_is_dir = b.path().is_dir();
                if a_is_dir != b_is_dir {
                    b_is_dir.cmp(&a_is_dir)
                } else {
                    a.file_name().cmp(&b.file_name())
                }
            });
            for entry in entries {
                html.push_str("<li>");
                render_html_recursive(root, &entry.path(), prefix, html);
                html.push_str("</li>");
            }
        }
        html.push_str("</ul></details>");
    } else if let Ok(rel_path) = current.strip_prefix(root) {
        let path_str = rel_path.to_string_lossy();
        let encoded_path = encode_path(&path_str);
        let icon = get_file_icon(current);
        let _ = write!(
            html,
            r#"<div class="file"><span class="file-icon">{}</span><a href="{}/{}">{}</a></div>"#,
            icon, prefix, encoded_path, name
        );
    }
}
