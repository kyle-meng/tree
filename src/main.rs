use axum::{
    extract::{Host, State},
    response::{Html, IntoResponse},
    routing::get,
    Json, Router,
};
use chrono::{DateTime, Local};
use clap::Parser;
use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};
use std::{
    fmt::Write,
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::Arc,
    time::SystemTime,
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

async fn handle_html_tree(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let mut html = String::with_capacity(16384);
    html.push_str(r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Tree Index</title>
    <style>
        body { font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif; padding: 20px; line-height: 1.6; color: #333; max-width: 1200px; margin: 0 auto; background-color: #f9f9f9; }
        h1 { border-bottom: 2px solid #eee; padding-bottom: 10px; color: #2c3e50; }
        ul { list-style-type: none; padding-left: 20px; margin: 0; }
        li { margin: 2px 0; }
        details { margin: 1px 0; }
        summary { cursor: pointer; color: #3498db; font-weight: 600; outline: none; list-style: none; display: flex; align-items: center; padding: 4px; border-radius: 4px; }
        summary::-webkit-details-marker { display: none; }
        summary:hover { background: #f0f0f0; }
        summary::before { content: "📁 "; display: inline-block; width: 1.5em; flex-shrink: 0; }
        details[open] > summary::before { content: "📂 "; }
        
        .item-row { display: flex; align-items: center; justify-content: space-between; padding: 4px 8px; border-radius: 4px; transition: background 0.2s; }
        .item-row:hover { background: #f0f0f0; }
        .file-info { display: flex; align-items: center; flex-grow: 1; overflow: hidden; }
        .file-info::before { content: "📄 "; display: inline-block; width: 1.5em; flex-shrink: 0; }
        .file-name { color: #2c3e50; text-decoration: none; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
        .file-meta { display: flex; font-size: 0.85em; color: #7f8c8d; flex-shrink: 0; margin-left: 20px; }
        .meta-size { width: 80px; text-align: right; font-family: monospace; }
        .meta-time { width: 150px; text-align: right; margin-left: 20px; }

        .container { background: white; padding: 30px; border-radius: 8px; box-shadow: 0 2px 10px rgba(0,0,0,0.05); }
        .search-box { margin-bottom: 20px; position: sticky; top: 0; background: white; padding: 10px 0; z-index: 100; border-bottom: 1px solid #eee; }
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
            const allItems = treeRoot.querySelectorAll('li');
            const allDetails = treeRoot.querySelectorAll('details');

            if (term === '') {
                allItems.forEach(li => li.style.display = '');
                allDetails.forEach(details => {
                    details.style.display = '';
                    details.open = true;
                });
                return;
            }

            allItems.forEach(li => li.style.display = 'none');
            allDetails.forEach(details => {
                details.style.display = 'none';
                details.open = false;
            });

            const rows = treeRoot.querySelectorAll('.item-row, summary');
            rows.forEach(row => {
                if (row.textContent.toLowerCase().includes(term)) {
                    let li = row.closest('li');
                    if (li) li.style.display = 'block';
                    
                    let parent = row.parentElement;
                    while (parent && parent !== treeRoot) {
                        if (parent.tagName === 'DETAILS') {
                            parent.style.display = 'block';
                            parent.open = true;
                        } else if (parent.tagName === 'LI' || parent.tagName === 'UL') {
                            parent.style.display = 'block';
                        }
                        parent = parent.parentElement;
                    }

                    if (row.tagName === 'SUMMARY') {
                        let details = row.parentElement;
                        details.style.display = 'block';
                        details.querySelectorAll(':scope > ul > li').forEach(childLi => childLi.style.display = 'block');
                    }
                }
            });
        });
    </script>
</body>
</html>"#);

    Html(html)
}

fn format_size(size: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if size >= GB {
        format!("{:.2} GB", size as f64 / GB as f64)
    } else if size >= MB {
        format!("{:.2} MB", size as f64 / MB as f64)
    } else if size >= KB {
        format!("{:.2} KB", size as f64 / KB as f64)
    } else {
        format!("{} B", size)
    }
}

fn format_time(time: SystemTime) -> String {
    let datetime: DateTime<Local> = time.into();
    datetime.format("%Y-%m-%d %H:%M").to_string()
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
        
        let (size_str, time_str) = if let Ok(metadata) = current.metadata() {
            (
                format_size(metadata.len()),
                format_time(metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH))
            )
        } else {
            ("-".to_string(), "-".to_string())
        };

        let _ = write!(
            html,
            r#"<div class="item-row">
                <div class="file-info">
                    <a class="file-name" href="{}/{}">{}</a>
                </div>
                <div class="file-meta">
                    <span class="meta-size">{}</span>
                    <span class="meta-time">{}</span>
                </div>
            </div>"#,
            prefix, encoded_path, name, size_str, time_str
        );
    }
}
