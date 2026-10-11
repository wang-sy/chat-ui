use crate::runtime::{display_path, home_dir, Runtime};
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tiny_http::{Header, Method, Request, Response, Server, StatusCode};

pub const PORT: u16 = 4317;

mod web_assets {
    include!(concat!(env!("OUT_DIR"), "/web_assets.rs"));
}

fn header(name: &str, value: &str) -> Header {
    Header::from_bytes(name, value).expect("valid static HTTP header")
}

pub fn allowed_origin(origin: &str, port: u16) -> bool {
    if [
        "tauri://localhost",
        "http://tauri.localhost",
        "https://tauri.localhost",
    ]
    .contains(&origin)
    {
        return true;
    }
    let Ok(url) = url::Url::parse(origin) else {
        return false;
    };
    url.scheme() == "http"
        && [Some("127.0.0.1"), Some("localhost"), Some("[::1]")].contains(&url.host_str())
        && [Some(port), Some(1420)].contains(&url.port())
        && url.path() == "/"
        && url.query().is_none()
        && url.username().is_empty()
        && url.password().is_none()
}

fn json_response(request: Request, status: u16, body: Value, origin: Option<&str>) {
    let mut response = Response::from_string(body.to_string())
        .with_status_code(StatusCode(status))
        .with_header(header("Content-Type", "application/json; charset=utf-8"))
        .with_header(header("Cache-Control", "no-store"))
        .with_header(header("X-Content-Type-Options", "nosniff"));
    if let Some(origin) = origin {
        response.add_header(header("Access-Control-Allow-Origin", origin));
        response.add_header(header("Vary", "Origin"));
    }
    let _ = request.respond(response);
}

fn serve_page(request: Request, path: &str) {
    let page_path = if path == "/" { "/index.html" } else { path };
    let asset = web_assets::WEB_ASSETS
        .iter()
        .find(|(name, _)| *name == page_path);
    let Some((name, bytes)) = asset else {
        json_response(request, 404, json!({"error":"页面不存在"}), None);
        return;
    };
    let content_type = match Path::new(name).extension().and_then(|s| s.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("js") => "text/javascript; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("ico") => "image/x-icon",
        Some("woff2") => "font/woff2",
        _ => "application/octet-stream",
    };
    let response = Response::from_data(*bytes)
        .with_header(header("Content-Type", content_type))
        .with_header(header("Cache-Control", "no-cache"))
        .with_header(header("X-Content-Type-Options", "nosniff"))
        .with_header(header("Referrer-Policy", "no-referrer"))
        .with_header(header("Content-Security-Policy", "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; connect-src 'self' http://127.0.0.1:4317; frame-ancestors 'none'"));
    let _ = request.respond(response);
}

pub fn directories(path: Option<&str>) -> Result<Value, String> {
    let initial = path.map(PathBuf::from).unwrap_or_else(home_dir);
    let path = std::fs::canonicalize(initial).map_err(|_| "目录不存在或无法读取")?;
    if !path.is_dir() {
        return Err("请选择文件夹".into());
    }
    let mut children = vec![];
    for entry in std::fs::read_dir(&path)
        .map_err(|_| "无法读取目录")?
        .flatten()
    {
        if entry.path().is_dir() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if !name.starts_with('.') {
                children.push(json!({"name":name,"path":display_path(&entry.path())}));
            }
        }
    }
    children.sort_by_key(|value| value["name"].as_str().unwrap_or("").to_lowercase());
    let mut roots = vec![json!({"name":"个人目录","path":home_dir()})];
    #[cfg(windows)]
    for drive in 'A'..='Z' {
        let root = format!("{drive}:\\");
        if Path::new(&root).is_dir() {
            roots.push(json!({"name":root,"path":root}));
        }
    }
    #[cfg(not(windows))]
    roots.push(json!({"name":"根目录","path":"/"}));
    Ok(
        json!({"path":display_path(&path),"parent":path.parent().map(display_path),"children":children,"roots":roots}),
    )
}

fn handle(mut request: Request, runtime: Arc<Runtime>, token: &str, port: u16) {
    let origin = request
        .headers()
        .iter()
        .find(|h| h.field.equiv("Origin"))
        .map(|h| h.value.to_string());
    let host = request
        .headers()
        .iter()
        .find(|h| h.field.equiv("Host"))
        .map(|h| h.value.to_string())
        .unwrap_or_default();
    if ![
        format!("127.0.0.1:{port}"),
        format!("localhost:{port}"),
        format!("[::1]:{port}"),
    ]
    .contains(&host)
        || origin.as_deref().is_some_and(|o| !allowed_origin(o, port))
    {
        json_response(request, 403, json!({"error":"不允许的来源"}), None);
        return;
    }
    if request.method() == &Method::Options {
        let mut response = Response::empty(204)
            .with_header(header(
                "Access-Control-Allow-Headers",
                "Content-Type, X-Chat-UI-Token",
            ))
            .with_header(header("Access-Control-Allow-Methods", "GET, POST, OPTIONS"));
        if let Some(origin) = &origin {
            response.add_header(header("Access-Control-Allow-Origin", origin));
        }
        let _ = request.respond(response);
        return;
    }
    let Ok(url) = url::Url::parse(&format!("http://127.0.0.1:{}{}", port, request.url())) else {
        json_response(
            request,
            400,
            json!({"error":"无效的 URL"}),
            origin.as_deref(),
        );
        return;
    };
    let path = url.path();
    if request.method() == &Method::Get && path == "/health" {
        json_response(
            request,
            200,
            json!({"service":"chat-ui","protocol":1,"token":token}),
            origin.as_deref(),
        );
        return;
    }
    if !path.starts_with("/api/") {
        if request.method() == &Method::Get {
            serve_page(request, path);
            return;
        }
        json_response(
            request,
            404,
            json!({"error":"Not found"}),
            origin.as_deref(),
        );
        return;
    }
    if request
        .headers()
        .iter()
        .find(|h| h.field.equiv("X-Chat-UI-Token"))
        .map(|h| h.value.as_str())
        != Some(token)
    {
        json_response(
            request,
            401,
            json!({"error":"请重新连接本地服务"}),
            origin.as_deref(),
        );
        return;
    }
    if request.method() == &Method::Get {
        let result = match path {
            "/api/state" => Ok(runtime.snapshot()),
            "/api/directories" => directories(
                url.query_pairs()
                    .find(|(k, _)| k == "path")
                    .as_ref()
                    .map(|(_, v)| v.as_ref()),
            ),
            _ => {
                json_response(
                    request,
                    404,
                    json!({"error":"Not found"}),
                    origin.as_deref(),
                );
                return;
            }
        };
        match result {
            Ok(body) => json_response(request, 200, body, origin.as_deref()),
            Err(error) => json_response(request, 400, json!({"error":error}), origin.as_deref()),
        }
        return;
    }
    if request.method() != &Method::Post {
        json_response(
            request,
            405,
            json!({"error":"Method not allowed"}),
            origin.as_deref(),
        );
        return;
    }
    let mut text = String::new();
    if request
        .as_reader()
        .take(1_048_577)
        .read_to_string(&mut text)
        .is_err()
        || text.len() > 1_048_576
    {
        json_response(request, 413, json!({"error":"请求过大"}), origin.as_deref());
        return;
    }
    let body: Value = match serde_json::from_str(&text) {
        Ok(value) => value,
        Err(_) => {
            json_response(
                request,
                400,
                json!({"error":"请求不是有效 JSON"}),
                origin.as_deref(),
            );
            return;
        }
    };
    let result = match path {
        "/api/connect" => runtime.connect().map(|_| runtime.snapshot()),
        "/api/projects" => runtime.add_project(body["path"].as_str().unwrap_or("")),
        "/api/chats/send" => runtime.send(&body),
        "/api/chats/open" => runtime.open_chat(body["id"].as_str().unwrap_or("")),
        "/api/chats/interrupt" => runtime.interrupt(body["id"].as_str().unwrap_or("")),
        "/api/requests/reply" => runtime.reply(&body),
        _ => {
            json_response(
                request,
                404,
                json!({"error":"Not found"}),
                origin.as_deref(),
            );
            return;
        }
    };
    match result {
        Ok(body) => json_response(request, 200, body, origin.as_deref()),
        Err(error) => json_response(request, 400, json!({"error":error}), origin.as_deref()),
    }
}

pub fn serve(port: u16, state_dir: PathBuf) -> Result<(), String> {
    let server = Server::http(("127.0.0.1", port)).map_err(|e| e.to_string())?;
    let runtime = Runtime::new(state_dir);
    let token = uuid::Uuid::new_v4().to_string();
    println!("Chat UI local service: http://127.0.0.1:{port}");
    for request in server.incoming_requests() {
        let runtime = runtime.clone();
        let token = token.clone();
        std::thread::spawn(move || handle(request, runtime, &token, port));
    }
    Ok(())
}

pub fn is_running(port: u16) -> bool {
    let Ok(address) = format!("127.0.0.1:{port}")
        .to_socket_addrs()
        .and_then(|mut a| a.next().ok_or_else(|| std::io::Error::other("no address")))
    else {
        return false;
    };
    let Ok(mut stream) = TcpStream::connect_timeout(&address, Duration::from_millis(200)) else {
        return false;
    };
    let _ = stream.set_read_timeout(Some(Duration::from_millis(500)));
    let _ = stream.write_all(
        format!("GET /health HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n")
            .as_bytes(),
    );
    let mut response = String::new();
    let _ = stream.read_to_string(&mut response);
    response.contains("\"service\":\"chat-ui\"") && response.contains("\"protocol\":1")
}

pub fn state_directory() -> PathBuf {
    std::env::var_os("CHAT_UI_STATE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| home_dir().join(".chat-ui"))
}
