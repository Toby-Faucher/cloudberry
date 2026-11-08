use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use server::{Config, app};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_tungstenite::tungstenite::Message;

async fn start(tag: &str) -> (String, std::path::PathBuf) {
    let dir = std::env::temp_dir().join(format!("cb-it-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let cfg = Config {
        music_dir: dir.join("music"),
        db_path: dir.join("db.sqlite"),
        bind: String::new(),
        static_dir: dir.join("missing-static"),
        ytdlp: "/nonexistent/yt-dlp".into(),
    };
    let (router, _) = app(cfg).unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap().to_string();
    tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    (addr, dir)
}

async fn http(addr: &str, req_lines: &str) -> String {
    let mut s = tokio::net::TcpStream::connect(addr).await.unwrap();
    let req = format!("{req_lines}\r\nHost: x\r\nConnection: close\r\n\r\n");
    s.write_all(req.as_bytes()).await.unwrap();
    let mut out = Vec::new();
    s.read_to_end(&mut out).await.unwrap();
    String::from_utf8_lossy(&out).into_owned()
}

async fn next_json<S>(ws: &mut S) -> Value
where
    S: StreamExt<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin,
{
    loop {
        if let Message::Text(t) = ws.next().await.unwrap().unwrap() {
            return serde_json::from_str(&t).unwrap();
        }
    }
}

#[tokio::test]
async fn websocket_flow() {
    let (addr, dir) = start("ws").await;
    let (mut ws, _) = tokio_tungstenite::connect_async(format!("ws://{addr}/ws?name=%20Toby%20"))
        .await
        .unwrap();

    let welcome = next_json(&mut ws).await;
    assert_eq!(welcome["type"], "welcome");
    let me = welcome["you"].as_u64().unwrap();

    let state = next_json(&mut ws).await;
    assert_eq!(state["type"], "state");
    let st = &state["state"];
    assert_eq!(st["listeners"], json!([{ "id": me, "name": "Toby" }]));
    for k in [
        "current",
        "queue",
        "playing",
        "position_ms",
        "anchor_server_ms",
        "library",
        "jobs",
    ] {
        assert!(st.get(k).is_some(), "missing {k}");
    }

    ws.send(Message::text(r#"{"type":"ping","t0":123.5}"#))
        .await
        .unwrap();
    let pong = next_json(&mut ws).await;
    assert_eq!(pong["type"], "pong");
    assert_eq!(pong["t0"], 123.5);
    assert!(pong["server_time"].as_f64().unwrap() > 1.7e12);

    ws.send(Message::text(
        r#"{"type":"add","url":"https://youtube.com.evil.com/watch?v=jNQXAC9IVzw"}"#,
    ))
    .await
    .unwrap();
    assert_eq!(next_json(&mut ws).await["type"], "error");

    ws.send(Message::text(r#"{"type":"enqueue","track_id":"nope"}"#))
        .await
        .unwrap();
    assert_eq!(next_json(&mut ws).await["type"], "error");

    // A valid add creates a job; the fake yt-dlp path makes it fail.
    ws.send(Message::text(
        r#"{"type":"add","url":"https://youtu.be/jNQXAC9IVzw"}"#,
    ))
    .await
    .unwrap();
    let mut saw_failed = false;
    for _ in 0..10 {
        let m = next_json(&mut ws).await;
        if m["type"] == "job" && m["job"]["status"] == "failed" {
            assert!(m["job"]["error"].as_str().unwrap().contains("yt-dlp"));
            assert_eq!(
                m["job"]["url"],
                "https://www.youtube.com/watch?v=jNQXAC9IVzw"
            );
            assert_eq!(m["job"]["added_by"], "Toby");
            saw_failed = true;
            break;
        }
    }
    assert!(saw_failed);

    // A second listener shows up in everyone's state.
    let (_ws2, _) = tokio_tungstenite::connect_async(format!("ws://{addr}/ws?name=Ann"))
        .await
        .unwrap();
    loop {
        let m = next_json(&mut ws).await;
        if m["type"] == "state" && m["state"]["listeners"].as_array().unwrap().len() == 2 {
            break;
        }
    }
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test]
async fn http_endpoints() {
    let (addr, dir) = start("http").await;

    for bad in [
        "",
        "name=",
        "name=%20%20",
        &format!("name={}", "x".repeat(33)),
    ] {
        let err = tokio_tungstenite::connect_async(format!("ws://{addr}/ws?{bad}"))
            .await
            .unwrap_err();
        assert!(err.to_string().contains("400"), "{bad}: {err}");
    }

    let r = http(&addr, "GET /api/state HTTP/1.1").await;
    assert!(r.starts_with("HTTP/1.1 200"), "{r}");
    assert!(r.contains("\"library\":[]"));

    std::fs::write(dir.join("music/abcdefghijk.mp3"), vec![7u8; 1000]).unwrap();
    let r = http(
        &addr,
        "GET /audio/abcdefghijk.mp3 HTTP/1.1\r\nRange: bytes=100-199",
    )
    .await;
    assert!(r.starts_with("HTTP/1.1 206"), "{r}");
    assert!(
        r.to_lowercase()
            .contains("cache-control: public, max-age=31536000, immutable")
    );
    assert!(
        r.to_lowercase()
            .contains("content-range: bytes 100-199/1000")
    );

    let r = http(&addr, "GET /audio/missing.mp3 HTTP/1.1").await;
    assert!(r.starts_with("HTTP/1.1 404"), "{r}");
    assert!(!r.to_lowercase().contains("immutable"));

    // Static dir missing: 404, no crash.
    let r = http(&addr, "GET /some/spa/route HTTP/1.1").await;
    assert!(r.starts_with("HTTP/1.1 404"), "{r}");

    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test]
async fn spa_fallback() {
    let (addr, dir) = start("spa").await;
    drop(addr);
    // Fresh server pointing at a real static dir.
    let site = dir.join("site");
    std::fs::create_dir_all(&site).unwrap();
    std::fs::write(site.join("index.html"), "<html>cloudberry</html>").unwrap();
    std::fs::write(site.join("app.js"), "js").unwrap();
    let cfg = Config {
        music_dir: dir.join("music"),
        db_path: dir.join("db2.sqlite"),
        bind: String::new(),
        static_dir: site,
        ytdlp: "yt-dlp".into(),
    };
    let (router, _) = app(cfg).unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap().to_string();
    tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });

    let r = http(&addr, "GET /app.js HTTP/1.1").await;
    assert!(r.starts_with("HTTP/1.1 200") && r.ends_with("js"), "{r}");
    let r = http(&addr, "GET /deep/link HTTP/1.1").await;
    assert!(
        r.starts_with("HTTP/1.1 200") && r.contains("cloudberry"),
        "{r}"
    );
    let _ = std::fs::remove_dir_all(dir);
}
