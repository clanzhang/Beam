use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::UNIX_EPOCH;

use axum::body::Body;
use axum::extract::{
    ws::{Message as WsMessage, WebSocket, WebSocketUpgrade},
    DefaultBodyLimit, FromRequest, Multipart, Path as AxumPath, Request, State,
};
use axum::http::{header, HeaderMap, Method, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::any;
use axum::Router;
use rand::distributions::Alphanumeric;
use rand::Rng;
use serde::Serialize;
use serde_json::json;
use futures_util::{SinkExt, StreamExt};
use tokio::io::AsyncWriteExt;
use tokio_util::io::ReaderStream;
use tower_http::cors::{Any, CorsLayer};

const MAX_FILENAME_LEN: usize = 200;

pub struct ServerState {
    pub token: String,
    pub dir: RwLock<PathBuf>,
    pub log_path: Option<PathBuf>,
    /// 文件变化广播频道：上传/删除/导入/换目录时发送 "files_changed"，WebSocket 客户端据此即时刷新
    pub notify: tokio::sync::broadcast::Sender<String>,
    /// 新文件到达回调（用于发系统通知），None 时跳过
    pub on_new_file: Option<Arc<dyn Fn(&str) + Send + Sync>>,
    /// 文字互传：最近一条共享文本 (内容, 时间戳)
    pub clipboard: RwLock<Option<(String, u64)>>,
    /// 写入系统剪贴板的回调（手机发文字 → 电脑剪贴板）
    pub on_clipboard: Option<Arc<dyn Fn(&str) + Send + Sync>>,
}

/// 写一行访问日志：同时输出到 stderr 和日志文件（便于排查手机端问题）
fn log_line(st: &ServerState, line: &str) {
    eprintln!("[airbox] {line}");
    if let Some(p) = &st.log_path {
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(p) {
            let _ = writeln!(f, "{line}");
        }
    }
}

#[derive(Serialize, Clone, Debug)]
pub struct FileInfo {
    pub name: String,
    pub size: u64,
    pub modified: u64,
}

pub fn build_router(state: Arc<ServerState>) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST, Method::DELETE, Method::OPTIONS])
        .allow_headers([header::CONTENT_TYPE]);
    Router::new()
        .route("/t/:token", any(gateway_no_rest))
        .route("/t/:token/", any(gateway_no_rest))
        .route("/t/:token/*rest", any(gateway))
        .layer(DefaultBodyLimit::disable())
        .layer(cors)
        .with_state(state)
}

// ---------- 网关：token 校验 + 手动路由 ----------

async fn gateway_no_rest(
    State(st): State<Arc<ServerState>>,
    AxumPath(token): AxumPath<String>,
    req: Request,
) -> Response {
    gateway_inner(&st, token, "", req).await
}

async fn gateway(
    State(st): State<Arc<ServerState>>,
    AxumPath((token, rest)): AxumPath<(String, String)>,
    req: Request,
) -> Response {
    gateway_inner(&st, token, &rest, req).await
}

async fn gateway_inner(st: &Arc<ServerState>, token: String, rest: &str, req: Request) -> Response {
    if token != st.token {
        return (StatusCode::NOT_FOUND, "Not Found").into_response();
    }
    let ua = req
        .headers()
        .get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("-")
        .to_string();
    log_line(st, &format!("REQ {} {} UA={}", req.method(), req.uri(), ua));
    match (req.method().clone(), rest) {
        (Method::GET, "api/files") => list_files(st).await,
        (Method::POST, "api/upload") => upload_file(st, req).await,
        (Method::GET, "api/ws") => ws_upgrade(st, req).await,
        (Method::GET, "api/clipboard") => get_clipboard(st).await,
        (Method::POST, "api/clipboard") => set_clipboard(st, req).await,
        (Method::GET, r) if r.starts_with("api/files/") => {
            download_file(st, &r["api/files/".len()..], req).await
        }
        (Method::DELETE, r) if r.starts_with("api/files/") => {
            delete_file(st, &r["api/files/".len()..]).await
        }
        (Method::GET, _) => mobile_page(),
        _ => (StatusCode::NOT_FOUND, "Not Found").into_response(),
    }
}

// ---------- 手机网页（内嵌静态页） ----------

fn mobile_page() -> Response {
    (
        [
            (header::CONTENT_TYPE, "text/html; charset=utf-8"),
            (header::CACHE_CONTROL, "no-store, no-cache, must-revalidate"),
        ],
        include_str!("../mobile/index.html"),
    )
        .into_response()
}

// ---------- WebSocket 实时推送 ----------

async fn ws_upgrade(st: &Arc<ServerState>, req: Request) -> Response {
    let st = st.clone();
    match WebSocketUpgrade::from_request(req, &()).await {
        Ok(upgrade) => upgrade.on_upgrade(move |socket| ws_loop(socket, st)),
        Err(rej) => rej.into_response(),
    }
}

async fn ws_loop(socket: WebSocket, st: Arc<ServerState>) {
    let mut rx = st.notify.subscribe();
    let (mut sender, mut receiver) = socket.split();
    // 连接建立后先推一次，让客户端立刻同步
    let _ = sender.send(WsMessage::Text("files_changed".to_string())).await;
    loop {
        tokio::select! {
            msg = rx.recv() => {
                match msg {
                    Ok(m) => {
                        if sender.send(WsMessage::Text(m.into())).await.is_err() {
                            break;
                        }
                    }
                    Err(_) => break, // Lagged/Closed
                }
            }
            incoming = receiver.next() => {
                match incoming {
                    Some(Ok(_)) => {} // 忽略客户端消息（保活/心跳）
                    _ => break,
                }
            }
        }
    }
}

// ---------- API ----------

async fn list_files(st: &Arc<ServerState>) -> Response {
    let dir = st.dir.read().unwrap().clone();
    let mut files: Vec<FileInfo> = Vec::new();
    let Ok(rd) = tokio::fs::read_dir(&dir).await else {
        return (StatusCode::INTERNAL_SERVER_ERROR, JsonErr("目录读取失败")).into_response();
    };
    let mut rd = rd;
    while let Ok(Some(entry)) = rd.next_entry().await {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with(".airbox-tmp-") {
            continue;
        }
        let Ok(meta) = entry.metadata().await else { continue };
        if !meta.is_file() {
            continue;
        }
        let modified = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);
        files.push(FileInfo {
            name,
            size: meta.len(),
            modified,
        });
    }
    files.sort_by(|a, b| b.modified.cmp(&a.modified));
    (StatusCode::OK, JsonOk(json!({ "files": files }))).into_response()
}

async fn upload_file(st: &Arc<ServerState>, req: Request) -> Response {
    let mut multipart = match Multipart::from_request(req, &()).await {
        Ok(m) => m,
        Err(rej) => return rej.into_response(),
    };

    let dir = st.dir.read().unwrap().clone();
    if let Err(e) = tokio::fs::create_dir_all(&dir).await {
        eprintln!("[airbox] 创建目录失败: {e}");
        return (StatusCode::INTERNAL_SERVER_ERROR, JsonErr("目录不可写")).into_response();
    }

    let mut saved: u32 = 0;
    let mut errors: u32 = 0;
    let mut saved_names: Vec<String> = Vec::new();

    while let Ok(Some(mut field)) = multipart.next_field().await {
        let Some(raw_name) = field.file_name().map(|s| s.to_string()) else {
            continue;
        };
        let Some(name) = sanitize_filename(&raw_name) else {
            errors += 1;
            continue;
        };
        let final_path = dedupe_path(&dir.join(&name));
        let tmp_path = dir.join(format!(".airbox-tmp-{}", random_token(8)));

        let result: Result<(), std::io::Error> = async {
            let mut file = tokio::fs::File::create(&tmp_path).await?;
            while let Some(chunk) = field.next().await {
                let chunk = chunk.map_err(|e| {
                    std::io::Error::new(std::io::ErrorKind::Other, e.to_string())
                })?;
                file.write_all(&chunk).await?;
            }
            file.sync_all().await?;
            drop(file);
            tokio::fs::rename(&tmp_path, &final_path).await?;
            Ok(())
        }
        .await;

        match result {
            Ok(()) => {
                saved += 1;
                saved_names.push(name.clone());
            }
            Err(e) => {
                eprintln!("[airbox] 保存 {name} 失败: {e}");
                let _ = tokio::fs::remove_file(&tmp_path).await;
                errors += 1;
            }
        }
    }

    if saved > 0 {
        let _ = st.notify.send("files_changed".to_string());
        if let Some(cb) = &st.on_new_file {
            for n in &saved_names {
                cb(n);
            }
        }
    }
    log_line(
        st,
        &format!("UPLOAD ok={saved} err={errors} names={}", saved_names.join(",")),
    );
    (StatusCode::OK, JsonOk(json!({ "ok": true, "saved": saved, "errors": errors }))).into_response()
}

async fn download_file(st: &Arc<ServerState>, encoded_name: &str, req: Request) -> Response {
    let Some(name) = decode_name(encoded_name) else {
        return (StatusCode::BAD_REQUEST, JsonErr("非法文件名")).into_response();
    };
    let dir = st.dir.read().unwrap().clone();
    let path = dir.join(&name);
    if !path.is_file() {
        return (StatusCode::NOT_FOUND, JsonErr("文件不存在")).into_response();
    }
    let Ok(file) = tokio::fs::File::open(&path).await else {
        return (StatusCode::INTERNAL_SERVER_ERROR, JsonErr("打开文件失败")).into_response();
    };
    let size = file.metadata().await.map(|m| m.len()).unwrap_or(0);

    // Range 支持：断点续传 + 大视频可拖动进度条（206 Partial Content）
    if let Some(range) = req
        .headers()
        .get(header::RANGE)
        .and_then(|v| v.to_str().ok())
    {
        if let Some((start, end)) = parse_range(range, size) {
            use tokio::io::{AsyncReadExt, AsyncSeekExt};
            let mut file = file;
            if file.seek(std::io::SeekFrom::Start(start)).await.is_err() {
                return (StatusCode::INTERNAL_SERVER_ERROR, JsonErr("读取失败")).into_response();
            }
            let length = end - start + 1;
            let stream = ReaderStream::new(file.take(length));
            let mut headers = HeaderMap::new();
            headers.insert(
                header::CONTENT_TYPE,
                header::HeaderValue::from_static(mime_for(&name)),
            );
            headers.insert(
                header::CONTENT_LENGTH,
                header::HeaderValue::from_str(&length.to_string()).unwrap(),
            );
            headers.insert(
                header::CONTENT_RANGE,
                header::HeaderValue::from_str(&format!("bytes {start}-{end}/{size}")).unwrap(),
            );
            headers.insert(header::ACCEPT_RANGES, header::HeaderValue::from_static("bytes"));
            return (StatusCode::PARTIAL_CONTENT, headers, Body::from_stream(stream)).into_response();
        } else {
            // 区间不合法 → 416
            return (
                StatusCode::RANGE_NOT_SATISFIABLE,
                [(header::CONTENT_RANGE, format!("bytes */{size}"))],
                "Range Not Satisfiable",
            )
                .into_response();
        }
    }

    let stream = ReaderStream::new(file);
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static(mime_for(&name)),
    );
    headers.insert(
        header::CONTENT_DISPOSITION,
        header::HeaderValue::from_str(&format!(
            "attachment; filename=\"{}\"",
            name.replace('"', "_")
        ))
        .unwrap_or_else(|_| header::HeaderValue::from_static("attachment")),
    );
    headers.insert(
        header::CONTENT_LENGTH,
        header::HeaderValue::from_str(&size.to_string()).unwrap(),
    );
    headers.insert(header::ACCEPT_RANGES, header::HeaderValue::from_static("bytes"));
    (headers, Body::from_stream(stream)).into_response()
}

/// 解析 Range 头：bytes=start-end / bytes=start- / bytes=-suffix。非法时返回 None（→ 416）
fn parse_range(range: &str, len: u64) -> Option<(u64, u64)> {
    let spec = range.strip_prefix("bytes=")?;
    let (s, e) = spec.split_once('-')?;
    if s.is_empty() {
        // bytes=-N：最后 N 个字节，end 固定为 len-1
        let n: u64 = e.parse().ok()?;
        if n == 0 {
            return None;
        }
        let start = len.saturating_sub(n);
        if start >= len {
            return None;
        }
        return Some((start, len - 1));
    }
    let start: u64 = s.parse().ok()?;
    let end: u64 = if e.is_empty() {
        len.saturating_sub(1)
    } else {
        e.parse::<u64>().ok()?.min(len.saturating_sub(1))
    };
    if start >= len || start > end {
        return None;
    }
    Some((start, end))
}

// ---------- 文字互传 ----------

async fn get_clipboard(st: &Arc<ServerState>) -> Response {
    let c = st.clipboard.read().unwrap();
    match &*c {
        Some((text, ts)) => (StatusCode::OK, JsonOk(json!({ "text": text, "updated_at": ts }))).into_response(),
        None => (StatusCode::OK, JsonOk(json!({ "text": "", "updated_at": 0 }))).into_response(),
    }
}

async fn set_clipboard(st: &Arc<ServerState>, req: Request) -> Response {
    let bytes = match axum::body::to_bytes(req.into_body(), 1024 * 1024).await {
        Ok(b) => b,
        Err(_) => return (StatusCode::BAD_REQUEST, JsonErr("请求体过大")).into_response(),
    };
    let text = serde_json::from_slice::<serde_json::Value>(&bytes)
        .ok()
        .and_then(|v| v.get("text").and_then(|t| t.as_str()).map(|s| s.to_string()))
        .unwrap_or_default();
    let ts = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    *st.clipboard.write().unwrap() = Some((text.clone(), ts));
    if let Some(cb) = &st.on_clipboard {
        cb(&text);
    }
    let _ = st.notify.send("clipboard_changed".to_string());
    (StatusCode::OK, JsonOk(json!({ "ok": true }))).into_response()
}

async fn delete_file(st: &Arc<ServerState>, encoded_name: &str) -> Response {
    let Some(name) = decode_name(encoded_name) else {
        return (StatusCode::BAD_REQUEST, JsonErr("非法文件名")).into_response();
    };
    let dir = st.dir.read().unwrap().clone();
    let path = dir.join(&name);
    match tokio::fs::remove_file(&path).await {
        Ok(()) => {
            let _ = st.notify.send("files_changed".to_string());
            (StatusCode::OK, JsonOk(json!({ "ok": true }))).into_response()
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            (StatusCode::NOT_FOUND, JsonErr("文件不存在")).into_response()
        }
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, JsonErr("删除失败")).into_response(),
    }
}

// ---------- 工具 ----------

struct JsonOk(serde_json::Value);
impl IntoResponse for JsonOk {
    fn into_response(self) -> Response {
        (StatusCode::OK, axum::Json(self.0)).into_response()
    }
}
struct JsonErr(&'static str);
impl IntoResponse for JsonErr {
    fn into_response(self) -> Response {
        (StatusCode::OK, axum::Json(json!({ "error": self.0 }))).into_response()
    }
}

/// 按扩展名返回 Content-Type（图片缩略图渲染需要正确类型）
fn mime_for(name: &str) -> &'static str {
    let ext = name.rsplit('.').next().unwrap_or("").to_lowercase();
    match ext.as_str() {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "heic" | "heif" => "image/heic",
        "bmp" => "image/bmp",
        "svg" => "image/svg+xml",
        "mp4" => "video/mp4",
        "mov" => "video/quicktime",
        "webm" => "video/webm",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "m4a" => "audio/mp4",
        "pdf" => "application/pdf",
        "txt" | "md" => "text/plain; charset=utf-8",
        "zip" => "application/zip",
        "json" => "application/json",
        _ => "application/octet-stream",
    }
}

/// 清洗文件名：拒绝空名/`.`/`..`/控制字符/超长/含路径分隔符，保证不能逃逸目录。
/// 手机浏览器选择文件时不会带上路径分隔符，直接拒绝更安全。
pub(crate) fn sanitize_filename(raw: &str) -> Option<String> {
    let base = raw.trim().to_string();
    if base.is_empty()
        || base == "."
        || base == ".."
        || base.len() > MAX_FILENAME_LEN
        || base.contains(['/', '\\'])
        || base.chars().any(|c| c.is_control())
    {
        return None;
    }
    Some(base)
}

/// 重名自动追加 " (1)"、" (2)"...
pub(crate) fn dedupe_path(path: &Path) -> PathBuf {
    let parent = path.parent().unwrap_or(Path::new("."));
    let file_stem = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    let ext = path
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();
    if !path.exists() {
        return path.to_path_buf();
    }
    for i in 1..10_000u32 {
        let candidate = parent.join(format!("{file_stem} ({i}){ext}"));
        if !candidate.exists() {
            return candidate;
        }
    }
    parent.join(format!(
        "{file_stem}-{}{ext}",
        std::time::SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0)
    ))
}

/// 解码 URL 编码的文件名（下载/删除用），再走一次清洗防 `%2F` 注入。
fn decode_name(encoded: &str) -> Option<String> {
    let decoded = urlencoding::decode(encoded).ok()?.into_owned();
    sanitize_filename(&decoded)
}

pub fn random_token(len: usize) -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(len)
        .map(char::from)
        .collect()
}

// ---------- 测试 ----------

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    pub(crate) fn test_state(dir: &Path) -> Arc<ServerState> {
        let (tx, _rx) = tokio::sync::broadcast::channel(64);
        Arc::new(ServerState {
            token: "testtok123".to_string(),
            dir: RwLock::new(dir.to_path_buf()),
            log_path: None,
            notify: tx,
            on_new_file: None,
            clipboard: RwLock::new(None),
            on_clipboard: None,
        })
    }

    async fn call(
        router: Router,
        method: Method,
        uri: &str,
        headers: &[(&str, &str)],
        body: Body,
    ) -> Response {
        let mut builder = axum::http::Request::builder().method(method).uri(uri);
        for (k, v) in headers {
            builder = builder.header(*k, *v);
        }
        router.oneshot(builder.body(body).unwrap()).await.unwrap()
    }

    async fn body_text(resp: Response) -> String {
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        String::from_utf8_lossy(&bytes).to_string()
    }

    fn multipart_body(files: &[(&str, &str)]) -> Body {
        let boundary = "testboundary123";
        let mut s = String::new();
        for (name, content) in files {
            s.push_str(&format!(
                "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{name}\"\r\nContent-Type: text/plain\r\n\r\n{content}\r\n"
            ));
        }
        s.push_str(&format!("--{boundary}--\r\n"));
        Body::from(s)
    }

    const CT: &[(&str, &str)] =
        &[("content-type", "multipart/form-data; boundary=testboundary123")];

    #[tokio::test]
    async fn wrong_token_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let router = build_router(super::tests::test_state(dir.path()));
        let resp = call(
            router,
            Method::GET,
            "/t/nottherighttoken/api/files",
            &[],
            Body::empty(),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn mobile_page_served() {
        let dir = tempfile::tempdir().unwrap();
        let router = build_router(super::tests::test_state(dir.path()));
        for uri in ["/t/testtok123/", "/t/testtok123"] {
            let resp = call(router.clone(), Method::GET, uri, &[], Body::empty()).await;
            assert_eq!(resp.status(), StatusCode::OK);
            let text = body_text(resp).await;
            assert!(text.contains("AirBox"), "手机页应包含 AirBox: {text}");
            assert!(text.contains("<!doctype html"));
        }
    }

    #[tokio::test]
    async fn empty_list() {
        let dir = tempfile::tempdir().unwrap();
        let router = build_router(super::tests::test_state(dir.path()));
        let resp = call(
            router,
            Method::GET,
            "/t/testtok123/api/files",
            &[],
            Body::empty(),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::OK);
        let text = body_text(resp).await;
        assert!(text.contains("\"files\":[]"), "应返回空列表: {text}");
    }

    #[tokio::test]
    async fn upload_then_list_and_download() {
        let dir = tempfile::tempdir().unwrap();
        let router = build_router(super::tests::test_state(dir.path()));

        // 上传两个文件
        let resp = call(
            router.clone(),
            Method::POST,
            "/t/testtok123/api/upload",
            CT,
            multipart_body(&[("a.txt", "hello airbox"), ("b.md", "# title")]),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::OK, "{}", body_text(resp).await);
        let text = body_text(resp).await;
        assert!(text.contains("\"saved\":2"), "应保存 2 个: {text}");

        // 列表
        let resp = call(
            router.clone(),
            Method::GET,
            "/t/testtok123/api/files",
            &[],
            Body::empty(),
        )
        .await;
        let text = body_text(resp).await;
        assert!(text.contains("a.txt") && text.contains("b.md"), "{text}");

        // 下载内容一致
        let resp = call(
            router.clone(),
            Method::GET,
            "/t/testtok123/api/files/a.txt",
            &[],
            Body::empty(),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::OK);
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(&bytes[..], b"hello airbox");

        // 磁盘上也真的写了
        assert_eq!(
            std::fs::read_to_string(dir.path().join("a.txt")).unwrap(),
            "hello airbox"
        );
    }

    #[tokio::test]
    async fn duplicate_name_gets_suffix() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("photo.jpg"), "v1").unwrap();
        let router = build_router(super::tests::test_state(dir.path()));
        let resp = call(
            router,
            Method::POST,
            "/t/testtok123/api/upload",
            CT,
            multipart_body(&[("photo.jpg", "v2")]),
        )
        .await;
        let text = body_text(resp).await;
        assert!(text.contains("\"saved\":1"), "{text}");
        assert_eq!(std::fs::read_to_string(dir.path().join("photo (1).jpg")).unwrap(), "v2");
    }

    #[tokio::test]
    async fn path_traversal_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let router = build_router(super::tests::test_state(dir.path()));
        // 带真实路径分隔符的文件名必须被拒绝，不能逃逸目录
        let resp = call(
            router.clone(),
            Method::POST,
            "/t/testtok123/api/upload",
            CT,
            multipart_body(&[("../../evil.txt", "x")]),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::OK);
        let text = body_text(resp).await;
        assert!(text.contains("\"saved\":0"), "应拒绝并保存 0 个: {text}");

        // multer 不解码 %2f，字面量文件名虽怪但无害（不含真实分隔符）
        let resp = call(
            router.clone(),
            Method::POST,
            "/t/testtok123/api/upload",
            CT,
            multipart_body(&[("%2e%2e%2fevil2.txt", "x")]),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::OK);

        // 列表里不应出现含真实分隔符的名字
        let resp = call(
            router.clone(),
            Method::GET,
            "/t/testtok123/api/files",
            &[],
            Body::empty(),
        )
        .await;
        let text = body_text(resp).await;
        assert!(!text.contains("/"), "列表不应含路径分隔符: {text}");

        // 目录外不应有文件
        let parent = dir.path().parent().unwrap();
        for entry in std::fs::read_dir(parent).unwrap() {
            if let Ok(e) = entry {
                let n = e.file_name().to_string_lossy().to_string();
                assert!(!n.contains("evil"), "目录穿越泄露: {n}");
            }
        }
    }

    #[tokio::test]
    async fn download_missing_404() {
        let dir = tempfile::tempdir().unwrap();
        let router = build_router(super::tests::test_state(dir.path()));
        let resp = call(
            router,
            Method::GET,
            "/t/testtok123/api/files/nope.txt",
            &[],
            Body::empty(),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn delete_removes_file() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("gone.txt"), "bye").unwrap();
        let router = build_router(super::tests::test_state(dir.path()));
        let resp = call(
            router.clone(),
            Method::DELETE,
            "/t/testtok123/api/files/gone.txt",
            &[],
            Body::empty(),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::OK);
        assert!(!dir.path().join("gone.txt").exists());
        // 再删一次 → 404
        let resp = call(
            router,
            Method::DELETE,
            "/t/testtok123/api/files/gone.txt",
            &[],
            Body::empty(),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn upload_broadcasts_files_changed() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_state(dir.path());
        let mut rx = state.notify.subscribe();
        let router = build_router(state);
        let resp = call(
            router,
            Method::POST,
            "/t/testtok123/api/upload",
            CT,
            multipart_body(&[("a.txt", "hi")]),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(rx.try_recv(), Ok("files_changed".to_string()));
    }

    #[tokio::test]
    async fn delete_broadcasts_files_changed() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("gone.txt"), "bye").unwrap();
        let state = test_state(dir.path());
        let mut rx = state.notify.subscribe();
        let router = build_router(state);
        let resp = call(
            router,
            Method::DELETE,
            "/t/testtok123/api/files/gone.txt",
            &[],
            Body::empty(),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(rx.try_recv(), Ok("files_changed".to_string()));
    }

    #[tokio::test]
    async fn range_download_returns_partial_content() {
        let dir = tempfile::tempdir().unwrap();
        let content = b"0123456789"; // 10 字节
        std::fs::write(dir.path().join("big.bin"), content).unwrap();
        let router = build_router(test_state(dir.path()));
        let resp = call(
            router.clone(),
            Method::GET,
            "/t/testtok123/api/files/big.bin",
            &[("range", "bytes=2-5")],
            Body::empty(),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::PARTIAL_CONTENT);
        assert_eq!(
            resp.headers().get(header::CONTENT_RANGE).and_then(|v| v.to_str().ok()),
            Some("bytes 2-5/10")
        );
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(&bytes[..], b"2345");

        // 后缀区间 bytes=-3 → 最后 3 字节
        let resp = call(
            router.clone(),
            Method::GET,
            "/t/testtok123/api/files/big.bin",
            &[("range", "bytes=-3")],
            Body::empty(),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::PARTIAL_CONTENT);
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(&bytes[..], b"789");

        // 超界 → 416
        let resp = call(
            router,
            Method::GET,
            "/t/testtok123/api/files/big.bin",
            &[("range", "bytes=50-60")],
            Body::empty(),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::RANGE_NOT_SATISFIABLE);
    }

    #[tokio::test]
    async fn clipboard_set_and_get() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_state(dir.path());
        let mut rx = state.notify.subscribe();
        let router = build_router(state);
        // 设置
        let resp = call(
            router.clone(),
            Method::POST,
            "/t/testtok123/api/clipboard",
            &[("content-type", "application/json")],
            Body::from(r#"{"text":"你好，AirBox"}"#),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(rx.try_recv(), Ok("clipboard_changed".to_string()));
        // 读取
        let resp = call(
            router,
            Method::GET,
            "/t/testtok123/api/clipboard",
            &[],
            Body::empty(),
        )
        .await;
        let text = body_text(resp).await;
        assert!(text.contains("你好，AirBox"), "{text}");
    }

    #[test]
    fn parse_range_variants() {
        assert_eq!(parse_range("bytes=2-5", 10), Some((2, 5)));
        assert_eq!(parse_range("bytes=5-", 10), Some((5, 9)));
        assert_eq!(parse_range("bytes=-3", 10), Some((7, 9)));
        assert_eq!(parse_range("bytes=0-999", 10), Some((0, 9)));
        assert_eq!(parse_range("bytes=50-60", 10), None);
        assert_eq!(parse_range("bytes=5-2", 10), None);
        assert_eq!(parse_range("items=0-1", 10), None);
    }

    #[test]
    fn mime_for_common_types() {
        assert_eq!(mime_for("a.jpg"), "image/jpeg");
        assert_eq!(mime_for("a.PNG"), "image/png");
        assert_eq!(mime_for("a.mp4"), "video/mp4");
        assert_eq!(mime_for("a.pdf"), "application/pdf");
        assert_eq!(mime_for("a.xyz"), "application/octet-stream");
    }

    #[test]
    fn sanitize_rejects_dangerous() {
        assert_eq!(sanitize_filename("../evil.txt"), None);
        assert_eq!(sanitize_filename(".."), None);
        assert_eq!(sanitize_filename("."), None);
        assert_eq!(sanitize_filename(""), None);
        assert_eq!(sanitize_filename("a/b.txt"), None);
        assert_eq!(sanitize_filename("a\\b.txt"), None);
        assert_eq!(sanitize_filename("  ok.txt  "), Some("ok.txt".to_string()));
        assert_eq!(sanitize_filename(&"x".repeat(300)), None);
    }
}

