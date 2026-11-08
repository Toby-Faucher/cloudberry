use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use server::{Config, app};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_tungstenite::tungstenite::Message;

fn fresh_dir(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("cb-it-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

async fn start(tag: &str) -> (String, std::path::PathBuf) {
    start_in(fresh_dir(tag)).await
}

async fn start_in(dir: std::path::PathBuf) -> (String, std::path::PathBuf) {
    start_with(dir, |_| {}).await
}

/// Start a server on `dir`, letting the test tweak the config first.
async fn start_with(
    dir: std::path::PathBuf,
    tweak: impl FnOnce(&mut Config),
) -> (String, std::path::PathBuf) {
    let mut cfg = Config {
        music_dir: dir.join("music"),
        db_path: dir.join("db.sqlite"),
        bind: String::new(),
        static_dir: dir.join("missing-static"),
        ytdlp: "/nonexistent/yt-dlp".into(),
        idle_timeout: std::time::Duration::from_secs(120),
        download_timeout: std::time::Duration::from_secs(900),
        playlist_timeout: std::time::Duration::from_secs(60),
    };
    tweak(&mut cfg);
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

    let history = next_json(&mut ws).await;
    assert_eq!(history, json!({ "type": "chat_history", "messages": [] }));

    // The library comes on its own, before the first state.
    let library = next_json(&mut ws).await;
    assert_eq!(library, json!({ "type": "library", "tracks": [] }));
    let playlists = next_json(&mut ws).await;
    assert_eq!(playlists, json!({ "type": "playlists", "playlists": [] }));

    let state = next_json(&mut ws).await;
    assert_eq!(state["type"], "state");
    let st = &state["state"];
    assert_eq!(
        st["listeners"],
        json!([{ "id": me, "name": "Toby", "away": false }])
    );
    assert_eq!(st["dj_of_day"], Value::Null);
    for k in [
        "current",
        "queue",
        "playing",
        "position_ms",
        "anchor_server_ms",
        "jobs",
        "history",
    ] {
        assert!(st.get(k).is_some(), "missing {k}");
    }
    assert!(st.get("library").is_none(), "library moved out of state");
    let joined = next_json(&mut ws).await;
    assert!(is_chat(&joined, "event", "Toby", "joined"), "{joined}");

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
    assert!(!r.contains("\"library\""), "{r}");
    let r = http(&addr, "GET /api/library HTTP/1.1").await;
    assert!(r.starts_with("HTTP/1.1 200"), "{r}");
    assert!(r.ends_with("\r\n\r\n[]"), "{r}");

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

    // Only {id}.mp3 / {id}.jpg are reachable: no temp/partial files, no cross-type access.
    std::fs::write(dir.join("music/abcdefghijk.jpg"), b"jpg").unwrap();
    std::fs::write(dir.join("music/abcdefghijk.mp3.part"), b"part").unwrap();
    std::fs::write(dir.join("music/abcdefghijk.webm"), b"webm").unwrap();
    std::fs::write(dir.join("music/notes.mp3"), b"x").unwrap();
    let r = http(&addr, "GET /art/abcdefghijk.jpg HTTP/1.1").await;
    assert!(
        r.starts_with("HTTP/1.1 200") && r.contains("image/jpeg"),
        "{r}"
    );
    for path in [
        "/audio/abcdefghijk.mp3.part",
        "/audio/abcdefghijk.webm",
        "/audio/abcdefghijk.jpg",
        "/art/abcdefghijk.mp3",
        "/audio/notes.mp3",
        "/audio/..%2Fdb.sqlite",
        "/audio/sub/abcdefghijk.mp3",
    ] {
        let r = http(&addr, &format!("GET {path} HTTP/1.1")).await;
        assert!(r.starts_with("HTTP/1.1 404"), "{path}: {r}");
    }

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
        idle_timeout: std::time::Duration::from_secs(120),
        download_timeout: std::time::Duration::from_secs(900),
        playlist_timeout: std::time::Duration::from_secs(60),
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

type Ws =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

async fn connect(addr: &str, name: &str) -> Ws {
    tokio_tungstenite::connect_async(format!("ws://{addr}/ws?name={name}"))
        .await
        .unwrap()
        .0
}

async fn send(ws: &mut Ws, v: Value) {
    ws.send(Message::text(v.to_string())).await.unwrap();
}

/// Read frames until one matches (5 s timeout).
async fn wait_for(ws: &mut Ws, pred: impl Fn(&Value) -> bool) -> Value {
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let m = next_json(ws).await;
            if pred(&m) {
                return m;
            }
        }
    })
    .await
    .expect("timed out waiting for message")
}

fn is_chat(m: &Value, kind: &str, name: &str, text: &str) -> bool {
    m["type"] == "chat"
        && m["message"]["kind"] == kind
        && m["message"]["name"] == name
        && m["message"]["text"] == text
}

#[tokio::test]
async fn chat_reactions_and_history_on_connect() {
    let (addr, dir) = start("chat").await;
    let mut a = connect(&addr, "Toby").await;
    let me = next_json(&mut a).await["you"].as_u64().unwrap();
    assert_eq!(next_json(&mut a).await["type"], "chat_history");
    assert_eq!(next_json(&mut a).await["type"], "library");
    assert_eq!(next_json(&mut a).await["type"], "playlists");
    assert_eq!(next_json(&mut a).await["type"], "state");
    let joined = next_json(&mut a).await;
    assert!(is_chat(&joined, "event", "Toby", "joined"), "{joined}");
    assert!(joined["message"]["at"].as_u64().unwrap() > 1_700_000_000_000);

    send(
        &mut a,
        json!({ "type": "chat", "text": "  hello there \n" }),
    )
    .await;
    let m = next_json(&mut a).await;
    assert!(is_chat(&m, "user", "Toby", "hello there"), "{m}");
    let hello_id = m["message"]["id"].as_u64().unwrap();

    for bad in ["   ", &"x".repeat(501)] {
        send(&mut a, json!({ "type": "chat", "text": bad })).await;
        assert_eq!(next_json(&mut a).await["type"], "error");
    }
    send(&mut a, json!({ "type": "chat", "text": "é".repeat(500) })).await;
    assert_eq!(next_json(&mut a).await["type"], "chat");

    send(&mut a, json!({ "type": "react", "emoji": "❤️" })).await;
    assert_eq!(
        next_json(&mut a).await,
        json!({ "type": "react", "listener_id": me, "name": "Toby", "emoji": "❤️" })
    );
    for bad in ["💩", "🔥🔥", ""] {
        send(&mut a, json!({ "type": "react", "emoji": bad })).await;
        assert_eq!(next_json(&mut a).await["type"], "error", "{bad}");
    }

    // A newcomer gets history (oldest first, without its own join), library, state, then its join.
    let mut b = connect(&addr, "Ann").await;
    assert_eq!(next_json(&mut b).await["type"], "welcome");
    let h = next_json(&mut b).await;
    assert_eq!(h["type"], "chat_history");
    let msgs = h["messages"].as_array().unwrap();
    assert_eq!(msgs.len(), 3);
    assert_eq!(msgs[0]["text"], "joined");
    assert_eq!(msgs[1]["id"], hello_id);
    assert_eq!(msgs[1]["kind"], "user");
    assert_eq!(next_json(&mut b).await["type"], "library");
    assert_eq!(next_json(&mut b).await["type"], "playlists");
    assert_eq!(next_json(&mut b).await["type"], "state");
    assert!(is_chat(&next_json(&mut b).await, "event", "Ann", "joined"));
    wait_for(&mut a, |m| is_chat(m, "event", "Ann", "joined")).await;

    b.close(None).await.unwrap();
    wait_for(&mut a, |m| is_chat(m, "event", "Ann", "left")).await;
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test]
async fn rate_limits_drop_excess_silently() {
    let (addr, dir) = start("rate").await;
    let mut a = connect(&addr, "Toby").await;
    wait_for(&mut a, |m| is_chat(m, "event", "Toby", "joined")).await;
    for i in 0..7 {
        send(&mut a, json!({ "type": "chat", "text": format!("m{i}") })).await;
    }
    for _ in 0..12 {
        send(&mut a, json!({ "type": "react", "emoji": "🔥" })).await;
    }
    // Direct replies can overtake our own broadcasts, so after the pong keep reading
    // until things go quiet.
    send(&mut a, json!({ "type": "ping", "t0": 1.0 })).await;
    let (mut chats, mut reacts, mut ponged) = (0, 0, false);
    let quiet = std::time::Duration::from_millis(300);
    while let Ok(m) = tokio::time::timeout(quiet, next_json(&mut a)).await {
        match m["type"].as_str().unwrap() {
            "chat" => chats += 1,
            "react" => reacts += 1,
            "pong" => ponged = true,
            t => panic!("unexpected {t}: {m}"),
        }
    }
    assert!(ponged);
    assert_eq!((chats, reacts), (5, 10));
    let _ = std::fs::remove_dir_all(dir);
}

const A: &str = "aaaaaaaaaaa";
const B: &str = "bbbbbbbbbbb";
const C: &str = "ccccccccccc";

fn queue_ids(st: &Value) -> Vec<String> {
    st["queue"]
        .as_array()
        .unwrap()
        .iter()
        .map(|q| q["track"]["id"].as_str().unwrap().to_string())
        .collect()
}

/// Seeds a DB in the MVP schema, then drives queue actions and checks events, insert and history.
#[tokio::test]
async fn mvp_db_migrates_and_actions_emit_events() {
    let dir = fresh_dir("events");
    std::fs::create_dir_all(dir.join("music")).unwrap();
    {
        let db = rusqlite::Connection::open(dir.join("db.sqlite")).unwrap();
        db.execute_batch(
            "CREATE TABLE tracks(id TEXT PRIMARY KEY, title TEXT NOT NULL,
                 duration_ms INTEGER NOT NULL, thumbnail INTEGER NOT NULL,
                 added_by TEXT NOT NULL, added_at INTEGER NOT NULL);
             CREATE TABLE queue(queue_id INTEGER PRIMARY KEY, track_id TEXT NOT NULL,
                 added_by TEXT NOT NULL, position INTEGER NOT NULL);
             CREATE TABLE station(k TEXT PRIMARY KEY, v TEXT NOT NULL);
             INSERT INTO tracks VALUES ('aaaaaaaaaaa', 'Song A', 600000, 0, 'toby', 1),
                                       ('bbbbbbbbbbb', 'Song B', 600000, 0, 'toby', 2),
                                       ('ccccccccccc', 'Song C', 600000, 0, 'toby', 3);",
        )
        .unwrap();
    }
    // With ffmpeg available, the startup backfill measures a real file (3 s 440 Hz sine).
    let have_ffmpeg = std::process::Command::new("ffmpeg")
        .args(["-loglevel", "error", "-f", "lavfi", "-i", "sine=f=440:d=3"])
        .arg(dir.join(format!("music/{A}.mp3")))
        .status()
        .is_ok_and(|s| s.success());
    let (addr, dir) = start_in(dir).await;
    let mut ws = connect(&addr, "Toby").await;
    let lib = wait_for(&mut ws, |m| m["type"] == "library").await;
    assert_eq!(lib["tracks"].as_array().unwrap().len(), 3);
    assert_eq!(lib["tracks"][0]["id"], C, "newest first");
    assert_eq!(lib["tracks"][0]["gain_db"], 0.0);
    if have_ffmpeg {
        // Measuring loudness changes the library, so it is re-sent.
        let m = wait_for(&mut ws, |m| {
            m["type"] == "library" && m["tracks"][2]["gain_db"] != 0.0
        })
        .await;
        let g = m["tracks"][2]["gain_db"].as_f64().unwrap();
        assert!((g - 8.2).abs() < 1.0, "gain {g}");
    } else {
        eprintln!("ffmpeg not found; skipping loudness backfill check");
    }

    for id in [A, B, C] {
        send(&mut ws, json!({ "type": "enqueue", "track_id": id })).await;
    }
    wait_for(&mut ws, |m| is_chat(m, "event", "Toby", "added Song C")).await;
    send(
        &mut ws,
        json!({ "type": "insert", "track_id": A, "index": 1 }),
    )
    .await;
    let st = wait_for(&mut ws, |m| m["type"] == "state").await;
    assert_eq!(queue_ids(&st["state"]), [B, A, C]);
    assert_eq!(st["state"]["current"]["track"]["id"], A);
    wait_for(&mut ws, |m| is_chat(m, "event", "Toby", "added Song A")).await;
    send(
        &mut ws,
        json!({ "type": "insert", "track_id": "nope", "index": 0 }),
    )
    .await;
    assert_eq!(
        wait_for(&mut ws, |m| m["type"] != "state").await["type"],
        "error"
    );

    send(&mut ws, json!({ "type": "pause" })).await;
    wait_for(&mut ws, |m| is_chat(m, "event", "Toby", "paused")).await;
    send(&mut ws, json!({ "type": "play" })).await;
    wait_for(&mut ws, |m| is_chat(m, "event", "Toby", "resumed")).await;

    send(&mut ws, json!({ "type": "skip" })).await;
    let st = wait_for(&mut ws, |m| m["type"] == "state").await;
    assert_eq!(st["state"]["current"]["track"]["id"], B);
    assert_eq!(st["state"]["history"][0]["id"], A);
    wait_for(&mut ws, |m| is_chat(m, "event", "Toby", "skipped Song A")).await;

    let qid = st["state"]["queue"][1]["queue_id"].clone(); // C
    send(&mut ws, json!({ "type": "remove", "queue_id": qid })).await;
    let st = wait_for(&mut ws, |m| m["type"] == "state").await;
    assert_eq!(queue_ids(&st["state"]), [A]);
    wait_for(&mut ws, |m| is_chat(m, "event", "Toby", "removed Song C")).await;

    // History and chat survive a restart on the same DB.
    let r = http(&start_in(dir.clone()).await.0, "GET /api/state HTTP/1.1").await;
    assert!(r.contains(r#""history":[{"id":"aaaaaaaaaaa""#), "{r}");
    let _ = std::fs::remove_dir_all(dir);
}

/// Seeds a DB in the phase-2 schema (no `reactions` table) with three measured tracks.
fn phase2_dir(tag: &str) -> std::path::PathBuf {
    let dir = fresh_dir(tag);
    std::fs::create_dir_all(dir.join("music")).unwrap();
    let db = rusqlite::Connection::open(dir.join("db.sqlite")).unwrap();
    db.execute_batch(
        "CREATE TABLE tracks(id TEXT PRIMARY KEY, title TEXT NOT NULL,
             duration_ms INTEGER NOT NULL, thumbnail INTEGER NOT NULL,
             added_by TEXT NOT NULL, added_at INTEGER NOT NULL, gain_db REAL);
         CREATE TABLE queue(queue_id INTEGER PRIMARY KEY, track_id TEXT NOT NULL,
             added_by TEXT NOT NULL, position INTEGER NOT NULL);
         CREATE TABLE station(k TEXT PRIMARY KEY, v TEXT NOT NULL);
         CREATE TABLE chat(id INTEGER PRIMARY KEY, kind TEXT NOT NULL,
             name TEXT NOT NULL, text TEXT NOT NULL, at INTEGER NOT NULL);
         CREATE TABLE history(track_id TEXT NOT NULL, played_at INTEGER NOT NULL);
         INSERT INTO tracks VALUES ('aaaaaaaaaaa', 'Song A', 600000, 0, 'zed', 1, 0),
                                   ('bbbbbbbbbbb', 'Song B', 600000, 0, 'zed', 2, 0),
                                   ('ccccccccccc', 'Song C', 600000, 0, 'zed', 3, 0);",
    )
    .unwrap();
    dir
}

/// Connect and read up to (and including) our own "joined" event; returns our listener id.
async fn join(addr: &str, name: &str) -> (Ws, u64) {
    let mut ws = connect(addr, name).await;
    let me = next_json(&mut ws).await["you"].as_u64().unwrap();
    wait_for(&mut ws, |m| is_chat(m, "event", name, "joined")).await;
    (ws, me)
}

fn is_dj(m: &Value, dj: Value) -> bool {
    m["type"] == "state" && m["state"]["dj_of_day"] == dj
}

#[tokio::test]
async fn poke_broadcasts_validates_and_rate_limits() {
    let (addr, dir) = start("poke").await;
    let (mut a, ia) = join(&addr, "Toby").await;
    let (mut b, ib) = join(&addr, "Ann").await;
    for bad in [ia, 9999] {
        send(&mut a, json!({ "type": "poke", "listener_id": bad })).await;
        wait_for(&mut a, |m| m["type"] == "error").await;
    }
    let poke = json!({ "type": "poke", "from_id": ia, "from_name": "Toby", "to_id": ib });
    for _ in 0..3 {
        send(&mut a, json!({ "type": "poke", "listener_id": ib })).await;
    }
    assert_eq!(wait_for(&mut b, |m| m["type"] == "poke").await, poke);
    send(&mut a, json!({ "type": "ping", "t0": 1.0 })).await;
    let (mut pokes, quiet) = (0, std::time::Duration::from_millis(300));
    while let Ok(m) = tokio::time::timeout(quiet, next_json(&mut a)).await {
        if m["type"] == "poke" {
            assert_eq!(m, poke);
            pokes += 1;
        }
    }
    assert_eq!(pokes, 1, "excess pokes are dropped");
    tokio::time::sleep(std::time::Duration::from_millis(2000)).await;
    send(&mut a, json!({ "type": "poke", "listener_id": ib })).await;
    assert_eq!(wait_for(&mut b, |m| m["type"] == "poke").await, poke);
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test]
async fn presence_sets_away() {
    let (addr, dir) = start("presence").await;
    let (mut a, ia) = join(&addr, "Toby").await;
    for away in [true, false] {
        send(&mut a, json!({ "type": "presence", "away": away })).await;
        let st = wait_for(&mut a, |m| m["type"] == "state").await;
        assert_eq!(
            st["state"]["listeners"],
            json!([{ "id": ia, "name": "Toby", "away": away }])
        );
    }
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test]
async fn last_listener_leaving_pauses() {
    let (addr, dir) = start_in(phase2_dir("autopause")).await;
    let (mut a, _) = join(&addr, "Toby").await;
    let (mut b, _) = join(&addr, "Ann").await;
    send(&mut a, json!({ "type": "enqueue", "track_id": A })).await;
    wait_for(&mut a, |m| {
        m["type"] == "state" && m["state"]["playing"] == true
    })
    .await;

    b.close(None).await.unwrap();
    wait_for(&mut a, |m| is_chat(m, "event", "Ann", "left")).await;
    let r = http(&addr, "GET /api/state HTTP/1.1").await;
    assert!(
        r.contains(r#""playing":true"#),
        "one of two leaving must not pause: {r}"
    );

    a.close(None).await.unwrap();
    drop(a);
    let mut paused = false;
    for _ in 0..50 {
        if http(&addr, "GET /api/state HTTP/1.1")
            .await
            .contains(r#""playing":false"#)
        {
            paused = true;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    assert!(paused);
    let mut c = connect(&addr, "Bob").await;
    let h = wait_for(&mut c, |m| m["type"] == "chat_history").await;
    let texts: Vec<_> = h["messages"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| {
            format!(
                "{}: {}",
                m["name"].as_str().unwrap(),
                m["text"].as_str().unwrap()
            )
        })
        .collect();
    assert_eq!(texts.last().unwrap(), "Cloudberry: paused", "{texts:?}");
    assert_eq!(texts.iter().filter(|t| t.ends_with("paused")).count(), 1);
    let _ = std::fs::remove_dir_all(dir);
}

/// Phase-2 DB gains the reactions table; reactions credit whoever queued the current track.
#[tokio::test]
async fn dj_of_day_from_phase2_db() {
    let (addr, dir) = start_in(phase2_dir("dj")).await;
    let (mut ann, _) = join(&addr, "Ann").await;
    let (mut bob, _) = join(&addr, "Bob").await;

    // Nothing current: the reaction is broadcast but not counted.
    send(&mut ann, json!({ "type": "react", "emoji": "🔥" })).await;
    wait_for(&mut ann, |m| m["type"] == "react").await;

    send(&mut ann, json!({ "type": "enqueue", "track_id": A })).await;
    wait_for(&mut bob, |m| {
        m["type"] == "state" && m["state"]["current"]["added_by"] == "Ann"
    })
    .await;
    send(&mut bob, json!({ "type": "enqueue", "track_id": B })).await;
    wait_for(&mut bob, |m| is_chat(m, "event", "Bob", "added Song B")).await;

    // Bob reacts to Ann's track: credit goes to Ann (the current item's added_by), not Bob.
    send(&mut bob, json!({ "type": "react", "emoji": "❤️" })).await;
    let st = wait_for(&mut bob, |m| m["type"] == "state").await;
    assert_eq!(
        st["state"]["dj_of_day"], "Ann",
        "first counted reaction: {st}"
    );
    send(&mut bob, json!({ "type": "react", "emoji": "❤️" })).await;
    wait_for(&mut bob, |m| m["type"] == "react").await; // Ann 2

    send(&mut ann, json!({ "type": "skip" })).await;
    wait_for(&mut ann, |m| {
        m["type"] == "state" && m["state"]["current"]["added_by"] == "Bob"
    })
    .await;
    send(&mut ann, json!({ "type": "react", "emoji": "🎉" })).await; // Bob 1
    wait_for(&mut ann, |m| m["type"] == "react").await;
    send(&mut ann, json!({ "type": "react", "emoji": "🎉" })).await; // Bob 2: tie, Bob is newer
    wait_for(&mut ann, |m| is_dj(m, json!("Bob"))).await;

    // Restart on the same DB: today's tally persists.
    let r = http(&start_in(dir.clone()).await.0, "GET /api/state HTTP/1.1").await;
    assert!(r.contains(r#""dj_of_day":"Bob""#), "{r}");
    let db = rusqlite::Connection::open(dir.join("db.sqlite")).unwrap();
    let rows: Vec<(String, i64)> = db
        .prepare("SELECT name, count FROM reactions ORDER BY name")
        .unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(rows, [("Ann".to_string(), 2), ("Bob".to_string(), 2)]);
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test]
async fn flood_of_queue_actions_is_limited() {
    let (addr, dir) = start_in(phase2_dir("flood")).await;
    let (mut a, _) = join(&addr, "Toby").await;
    for _ in 0..15 {
        send(&mut a, json!({ "type": "enqueue", "track_id": A })).await;
    }
    send(&mut a, json!({ "type": "ping", "t0": 1.0 })).await;
    let (mut added, quiet) = (0, std::time::Duration::from_millis(300));
    while let Ok(m) = tokio::time::timeout(quiet, next_json(&mut a)).await {
        assert_ne!(m["type"], "error", "excess is dropped silently: {m}");
        if is_chat(&m, "event", "Toby", "added Song A") {
            added += 1;
        }
    }
    assert_eq!(added, 10);
    let r = http(&addr, "GET /api/state HTTP/1.1").await;
    assert_eq!(r.matches(r#""queue_id""#).count(), 10, "current + 9 queued");
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test]
async fn silent_client_is_dropped() {
    let (addr, dir) = start_with(fresh_dir("idle"), |c| {
        c.idle_timeout = std::time::Duration::from_millis(500)
    })
    .await;
    let (mut silent, _) = join(&addr, "Ghost").await;
    let (mut live, _) = join(&addr, "Toby").await;
    let started = std::time::Instant::now();
    let mut left = false;
    // The live client pings (like the real one) while the silent one says nothing.
    while !left && started.elapsed() < std::time::Duration::from_secs(5) {
        send(&mut live, json!({ "type": "ping", "t0": 1.0 })).await;
        let tick = std::time::Duration::from_millis(100);
        while let Ok(m) = tokio::time::timeout(tick, next_json(&mut live)).await {
            left |= is_chat(&m, "event", "Ghost", "left");
        }
    }
    assert!(left, "silent client was not dropped");
    assert!(started.elapsed() >= std::time::Duration::from_millis(400));
    // The silent socket is closed; the pinging one is still served.
    let closed = tokio::time::timeout(std::time::Duration::from_secs(2), async {
        while let Some(Ok(m)) = silent.next().await {
            if m.is_close() {
                break;
            }
        }
    })
    .await;
    assert!(closed.is_ok());
    send(&mut live, json!({ "type": "ping", "t0": 2.0 })).await;
    wait_for(&mut live, |m| m["type"] == "pong").await;
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test]
async fn cross_site_websocket_is_refused() {
    use tokio_tungstenite::tungstenite::client::IntoClientRequest;
    let (addr, dir) = start("origin").await;
    let try_origin = |origin: Option<String>| {
        let mut req = format!("ws://{addr}/ws?name=Toby")
            .into_client_request()
            .unwrap();
        if let Some(o) = origin {
            req.headers_mut().insert("origin", o.parse().unwrap());
        }
        tokio_tungstenite::connect_async(req)
    };
    assert!(try_origin(None).await.is_ok(), "no Origin (CLI)");
    assert!(try_origin(Some(format!("http://{addr}"))).await.is_ok());
    for bad in [
        "http://evil.example".to_string(),
        format!("http://{addr}.evil.example"),
        "null".to_string(),
    ] {
        let err = try_origin(Some(bad.clone())).await.unwrap_err();
        assert!(err.to_string().contains("403"), "{bad}: {err}");
    }
    let _ = std::fs::remove_dir_all(dir);
}

/// A timed-out download kills yt-dlp's whole process tree and cleans up partial files.
#[tokio::test]
async fn download_timeout_kills_process_tree() {
    use std::os::unix::fs::PermissionsExt;
    let dir = fresh_dir("ytkill");
    std::fs::create_dir_all(&dir).unwrap();
    let (pids, script) = (dir.join("pids"), dir.join("fake-yt-dlp"));
    let music = dir.join("music");
    std::fs::write(
        &script,
        format!(
            "#!/bin/sh\necho $$ > {p}\nsleep 300 &\necho $! >> {p}\n\
             touch {m}/jNQXAC9IVzw.webm.part {m}/jNQXAC9IVzw.jpg\nsleep 300\n",
            p = pids.display(),
            m = music.display()
        ),
    )
    .unwrap();
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    let (addr, dir) = start_with(dir, |c| {
        c.ytdlp = script.display().to_string();
        c.download_timeout = std::time::Duration::from_secs(1);
    })
    .await;
    let (mut ws, _) = join(&addr, "Toby").await;
    send(
        &mut ws,
        json!({ "type": "add", "url": "https://youtu.be/jNQXAC9IVzw" }),
    )
    .await;
    let m = wait_for(&mut ws, |m| {
        m["type"] == "job" && m["job"]["status"] == "failed"
    })
    .await;
    assert_eq!(m["job"]["error"], "download timed out");

    let pids: Vec<String> = std::fs::read_to_string(&pids)
        .unwrap()
        .lines()
        .map(String::from)
        .collect();
    assert_eq!(pids.len(), 2, "script and its child");
    // Gone, or a zombie awaiting its (non-us) reaper: either way, not running.
    let running = |pid: &String| {
        std::fs::read_to_string(format!("/proc/{pid}/stat"))
            .is_ok_and(|s| !s.rsplit(')').next().unwrap_or("").starts_with(" Z"))
    };
    for _ in 0..40 {
        if !pids.iter().any(running) {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    assert!(!pids.iter().any(running), "still running: {pids:?}");
    let left: Vec<_> = std::fs::read_dir(&music)
        .unwrap()
        .flatten()
        .map(|e| e.file_name())
        .collect();
    assert!(left.is_empty(), "partials left: {left:?}");
    let _ = std::fs::remove_dir_all(dir);
}

/// Playlist import end to end with a fake yt-dlp: `--flat-playlist` prints the captured fixture
/// (or fails / hangs for special list ids); downloads just hang so jobs stay pending.
#[tokio::test]
async fn playlist_import_cancel_and_errors() {
    use std::os::unix::fs::PermissionsExt;
    let dir = fresh_dir("playlist");
    std::fs::create_dir_all(&dir).unwrap();
    let fixture = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/flat_playlist.json"
    );
    let script = dir.join("fake-yt-dlp");
    std::fs::write(
        &script,
        format!(
            "#!/bin/sh\n[ \"$1\" = --flat-playlist ] || exec sleep 30\n\
             case \"$7\" in\n\
             *list=PLBAD) echo 'ERROR: [youtube:tab] PLBAD: The playlist does not exist.' >&2; exit 1;;\n\
             *list=PLSLOW) exec sleep 30;;\n\
             esac\ncat {fixture}\n"
        ),
    )
    .unwrap();
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    // The first fixture entry is already in the library: it is queued without a job.
    {
        let db = rusqlite::Connection::open(dir.join("db.sqlite")).unwrap();
        db.execute_batch(
            "CREATE TABLE tracks(id TEXT PRIMARY KEY, title TEXT NOT NULL,
                 duration_ms INTEGER NOT NULL, thumbnail INTEGER NOT NULL,
                 added_by TEXT NOT NULL, added_at INTEGER NOT NULL, gain_db REAL);
             INSERT INTO tracks VALUES ('NOaZVn2ML44', 'Good intro Music', 16000, 0, 'ann', 1, 0);",
        )
        .unwrap();
    }
    let (addr, dir) = start_with(dir, |c| {
        c.ytdlp = script.display().to_string();
        c.playlist_timeout = std::time::Duration::from_secs(1);
    })
    .await;
    let (mut ws, _) = join(&addr, "Toby").await;
    let list = "PLKM3pENfZPOLCUSim2fut61AWCp9AUrQU";
    let url = format!("https://music.youtube.com/playlist?list={list}");
    send(&mut ws, json!({ "type": "add", "url": url })).await;
    let st = wait_for(&mut ws, |m| {
        m["type"] == "state" && m["state"]["jobs"].as_array().unwrap().len() == 4
    })
    .await;
    let st = &st["state"];
    assert_eq!(st["current"]["track"]["id"], "NOaZVn2ML44");
    assert_eq!(
        st["jobs"][0]["playlist"],
        json!({ "id": list, "title": "10 sec Intro Music" })
    );
    assert_eq!(st["jobs"][0]["title"], "Nice intro music"); // prefilled from the flat entry
    wait_for(&mut ws, |m| {
        is_chat(
            m,
            "event",
            "Toby",
            "added playlist 10 sec Intro Music (5 songs, 0 skipped) (first 5)",
        )
    })
    .await;

    // Cancel drops the queued jobs; the one downloading carries on.
    send(
        &mut ws,
        json!({ "type": "cancel_playlist", "playlist_id": list }),
    )
    .await;
    let st = wait_for(&mut ws, |m| {
        m["type"] == "state" && m["state"]["jobs"].as_array().unwrap().len() == 1
    })
    .await;
    assert_eq!(st["state"]["jobs"][0]["status"], "downloading");
    wait_for(&mut ws, |m| {
        is_chat(m, "event", "Toby", "cancelled playlist 10 sec Intro Music")
    })
    .await;
    send(
        &mut ws,
        json!({ "type": "cancel_playlist", "playlist_id": list }),
    )
    .await;
    assert_eq!(
        wait_for(&mut ws, |m| m["type"] == "error").await["message"],
        "Nothing left to cancel"
    );

    // Expansion failures come back to the sender as errors.
    let bad = "https://www.youtube.com/playlist?list=PLBAD";
    send(&mut ws, json!({ "type": "add", "url": bad })).await;
    assert_eq!(
        wait_for(&mut ws, |m| m["type"] == "error").await["message"],
        "Couldn't load that playlist: [youtube:tab] PLBAD: The playlist does not exist."
    );
    let slow = "https://www.youtube.com/playlist?list=PLSLOW";
    send(&mut ws, json!({ "type": "add", "url": slow })).await;
    assert_eq!(
        wait_for(&mut ws, |m| m["type"] == "error").await["message"],
        "Loading the playlist timed out"
    );
    let _ = std::fs::remove_dir_all(dir);
}

// ---------- saved shared playlists ----------

/// A DB in the schema as it was before playlists (phase 3: with `reactions`), three tracks.
fn pre_playlists_dir(tag: &str) -> std::path::PathBuf {
    let dir = phase2_dir(tag);
    let db = rusqlite::Connection::open(dir.join("db.sqlite")).unwrap();
    db.execute_batch(
        "CREATE TABLE reactions(day TEXT NOT NULL, name TEXT NOT NULL,
             count INTEGER NOT NULL, last_at INTEGER NOT NULL, PRIMARY KEY(day, name));",
    )
    .unwrap();
    dir
}

fn is_playlists(m: &Value) -> bool {
    m["type"] == "playlists"
}

fn tracks_of(m: &Value, i: usize) -> Vec<String> {
    m["playlists"][i]["track_ids"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t.as_str().unwrap().to_string())
        .collect()
}

async fn api_playlists(addr: &str) -> Value {
    let r = http(addr, "GET /api/playlists HTTP/1.1").await;
    assert!(r.starts_with("HTTP/1.1 200"), "{r}");
    serde_json::from_str(r.split("\r\n\r\n").nth(1).unwrap()).unwrap()
}

#[tokio::test]
async fn playlists_migrate_crud_queue_and_persist() {
    let (addr, dir) = start_in(pre_playlists_dir("plcrud")).await;
    // Connect order: welcome, chat_history, library, playlists, state.
    let mut ws = connect(&addr, "Toby").await;
    assert_eq!(next_json(&mut ws).await["type"], "welcome");
    assert_eq!(next_json(&mut ws).await["type"], "chat_history");
    assert_eq!(next_json(&mut ws).await["type"], "library");
    assert_eq!(
        next_json(&mut ws).await,
        json!({ "type": "playlists", "playlists": [] })
    );
    assert_eq!(next_json(&mut ws).await["type"], "state");
    wait_for(&mut ws, |m| is_chat(m, "event", "Toby", "joined")).await;

    send(
        &mut ws,
        json!({ "type": "playlist_create", "name": "  Road trip ", "track_ids": [A, "nope", B] }),
    )
    .await;
    let m = wait_for(&mut ws, is_playlists).await;
    let p = &m["playlists"][0];
    assert_eq!(p["name"], "Road trip");
    assert_eq!(p["created_by"], "Toby");
    assert_eq!(p["source_list_id"], Value::Null);
    assert!(p["created_at"].as_u64().unwrap() > 1_700_000_000_000);
    assert_eq!(tracks_of(&m, 0), [A, B], "unknown ids skipped");
    let id = p["id"].clone();
    wait_for(&mut ws, |m| {
        is_chat(m, "event", "Toby", "created playlist Road trip")
    })
    .await;

    send(
        &mut ws,
        json!({ "type": "playlist_add", "id": id, "track_ids": [C, A] }),
    )
    .await;
    let m = wait_for(&mut ws, is_playlists).await;
    assert_eq!(tracks_of(&m, 0), [A, B, C, A], "duplicates allowed");
    send(
        &mut ws,
        json!({ "type": "playlist_move", "id": id, "from": 0, "to": 2 }),
    )
    .await;
    let m = wait_for(&mut ws, is_playlists).await;
    assert_eq!(tracks_of(&m, 0), [B, C, A, A]);
    send(
        &mut ws,
        json!({ "type": "playlist_move", "id": id, "from": 3, "to": 99 }),
    )
    .await;
    let m = wait_for(&mut ws, is_playlists).await;
    assert_eq!(tracks_of(&m, 0), [B, C, A, A], "to is clamped");
    send(
        &mut ws,
        json!({ "type": "playlist_remove", "id": id, "index": 1 }),
    )
    .await;
    let m = wait_for(&mut ws, is_playlists).await;
    assert_eq!(tracks_of(&m, 0), [B, A, A]);
    send(
        &mut ws,
        json!({ "type": "playlist_rename", "id": id, "name": "Drive" }),
    )
    .await;
    wait_for(&mut ws, |m| {
        is_chat(m, "event", "Toby", "renamed playlist Road trip to Drive")
    })
    .await;

    send(
        &mut ws,
        json!({ "type": "playlist_queue", "id": id, "shuffle": false }),
    )
    .await;
    let st = wait_for(&mut ws, |m| m["type"] == "state").await;
    assert_eq!(
        st["state"]["current"]["track"]["id"], B,
        "idle station starts"
    );
    assert_eq!(st["state"]["current"]["added_by"], "Toby");
    assert_eq!(queue_ids(&st["state"]), [A, A]);
    wait_for(&mut ws, |m| {
        is_chat(m, "event", "Toby", "queued playlist Drive (3 songs)")
    })
    .await;
    send(
        &mut ws,
        json!({ "type": "playlist_queue", "id": id, "shuffle": true }),
    )
    .await;
    let st = wait_for(&mut ws, |m| m["type"] == "state").await;
    let mut q = queue_ids(&st["state"]);
    assert_eq!(q.len(), 5);
    q.sort();
    assert_eq!(q, [A, A, A, A, B]);
    wait_for(&mut ws, |m| {
        is_chat(m, "event", "Toby", "shuffled playlist Drive (3 songs)")
    })
    .await;

    // Newest-updated first.
    send(
        &mut ws,
        json!({ "type": "playlist_create", "name": "Empty" }),
    )
    .await;
    let m = wait_for(&mut ws, is_playlists).await;
    assert_eq!(m["playlists"][0]["name"], "Empty");
    assert_eq!(m["playlists"][1]["name"], "Drive");
    let api = api_playlists(&addr).await;
    assert_eq!(api, m["playlists"]);

    // Survives a restart on the same DB.
    let addr2 = start_in(dir.clone()).await.0;
    assert_eq!(api_playlists(&addr2).await, m["playlists"]);

    // A new connection (fresh action budget) deletes one.
    let (mut b, _) = join(&addr, "Ann").await;
    send(&mut b, json!({ "type": "playlist_delete", "id": id })).await;
    let m = wait_for(&mut b, is_playlists).await;
    assert_eq!(m["playlists"].as_array().unwrap().len(), 1);
    wait_for(&mut b, |m| {
        is_chat(m, "event", "Ann", "deleted playlist Drive")
    })
    .await;
    let addr3 = start_in(dir.clone()).await.0;
    let after = api_playlists(&addr3).await;
    assert_eq!(after.as_array().unwrap().len(), 1);
    assert_eq!(after[0]["name"], "Empty");
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test]
async fn playlist_errors_and_limits() {
    let dir = pre_playlists_dir("plerr");
    start_in(dir.clone()).await; // creates the tables
    {
        let db = rusqlite::Connection::open(dir.join("db.sqlite")).unwrap();
        // 100 playlists: #1 holds one song, #2 is full (500), #3 is empty.
        for i in 1..=100 {
            db.execute(
                "INSERT INTO playlists VALUES (?1, ?2, 'zed', 1, ?1, NULL)",
                rusqlite::params![i, format!("P{i}")],
            )
            .unwrap();
        }
        db.execute("INSERT INTO playlist_tracks VALUES (1, 0, ?1)", [A])
            .unwrap();
        for pos in 0..500 {
            db.execute(
                "INSERT INTO playlist_tracks VALUES (2, ?1, ?2)",
                rusqlite::params![pos, B],
            )
            .unwrap();
        }
    }
    let (addr, dir) = start_in(dir).await;

    let bad_name = "Playlist name must be 1–60 characters";
    let long = "x".repeat(61);
    let cases = [
        (
            json!({ "type": "playlist_create", "name": "ok" }),
            "Too many playlists",
        ),
        (
            json!({ "type": "playlist_create", "name": "   " }),
            bad_name,
        ),
        (
            json!({ "type": "playlist_rename", "id": 1, "name": long }),
            bad_name,
        ),
        (
            json!({ "type": "playlist_rename", "id": 999, "name": "x" }),
            "Unknown playlist",
        ),
        (
            json!({ "type": "playlist_delete", "id": 999 }),
            "Unknown playlist",
        ),
        (
            json!({ "type": "playlist_add", "id": 999, "track_ids": [A] }),
            "Unknown playlist",
        ),
        (
            json!({ "type": "playlist_add", "id": 2, "track_ids": [A] }),
            "Playlist is full",
        ),
        (
            json!({ "type": "playlist_add", "id": 1, "track_ids": ["nope"] }),
            "Unknown track",
        ),
        (
            json!({ "type": "playlist_remove", "id": 999, "index": 0 }),
            "Unknown playlist",
        ),
        (
            json!({ "type": "playlist_remove", "id": 1, "index": 1 }),
            "No such song in that playlist",
        ),
        (
            json!({ "type": "playlist_move", "id": 999, "from": 0, "to": 0 }),
            "Unknown playlist",
        ),
        (
            json!({ "type": "playlist_move", "id": 1, "from": 5, "to": 0 }),
            "No such song in that playlist",
        ),
        (
            json!({ "type": "playlist_queue", "id": 999, "shuffle": false }),
            "Unknown playlist",
        ),
        (
            json!({ "type": "playlist_queue", "id": 3, "shuffle": true }),
            "That playlist is empty",
        ),
        (
            json!({ "type": "playlist_create", "name": "y".repeat(60) }),
            "Too many playlists",
        ),
    ];
    // Errors count against the action limiter (10 / 5 s), so spread them over connections.
    for chunk in cases.chunks(5) {
        let (mut ws, _) = join(&addr, "Toby").await;
        for (msg, err) in chunk {
            send(&mut ws, msg.clone()).await;
            let m = wait_for(&mut ws, |m| m["type"] == "error").await;
            assert_eq!(m["message"], *err, "{msg}");
        }
    }

    // Every playlist message counts against the per-connection action limiter.
    let (mut ws, _) = join(&addr, "Flood").await;
    for i in 0..12 {
        send(
            &mut ws,
            json!({ "type": "playlist_rename", "id": 1, "name": format!("R{i}") }),
        )
        .await;
    }
    send(&mut ws, json!({ "type": "ping", "t0": 1.0 })).await;
    let (mut renamed, quiet) = (0, std::time::Duration::from_millis(300));
    while let Ok(m) = tokio::time::timeout(quiet, next_json(&mut ws)).await {
        assert_ne!(m["type"], "error", "{m}");
        if m["type"] == "chat"
            && m["message"]["text"]
                .as_str()
                .unwrap()
                .starts_with("renamed")
        {
            renamed += 1;
        }
    }
    assert_eq!(renamed, 10);
    let _ = std::fs::remove_dir_all(dir);
}

#[tokio::test]
async fn playlist_queue_stops_at_the_queue_cap() {
    let dir = pre_playlists_dir("plqcap");
    start_in(dir.clone()).await;
    {
        let db = rusqlite::Connection::open(dir.join("db.sqlite")).unwrap();
        for i in 1..=199 {
            db.execute(
                "INSERT INTO queue VALUES (?1, ?2, 'zed', ?1)",
                rusqlite::params![i, A],
            )
            .unwrap();
        }
        db.execute(
            r#"INSERT INTO station VALUES ('current', '{"queue_id":500,"track_id":"aaaaaaaaaaa","added_by":"zed"}')"#,
            [],
        )
        .unwrap();
        db.execute(
            "INSERT INTO playlists VALUES (7, 'Five', 'zed', 1, 1, NULL)",
            [],
        )
        .unwrap();
        for (pos, t) in [B, C, A, B, C].iter().enumerate() {
            db.execute(
                "INSERT INTO playlist_tracks VALUES (7, ?1, ?2)",
                rusqlite::params![pos, t],
            )
            .unwrap();
        }
    }
    let (addr, dir) = start_in(dir).await;
    let (mut ws, _) = join(&addr, "Toby").await;
    send(
        &mut ws,
        json!({ "type": "playlist_queue", "id": 7, "shuffle": false }),
    )
    .await;
    // The notice is a direct reply, so it may overtake the state broadcast.
    let (mut st, mut notice) = (Value::Null, Value::Null);
    while st.is_null() || notice.is_null() {
        let m = wait_for(&mut ws, |m| m["type"] == "state" || m["type"] == "error").await;
        if m["type"] == "state" {
            st = m;
        } else {
            notice = m;
        }
    }
    assert_eq!(queue_ids(&st["state"]).len(), 200);
    assert_eq!(queue_ids(&st["state"])[199], B, "the first song fit");
    assert_eq!(notice["message"], "Queued 1 of 5, queue is full");
    wait_for(&mut ws, |m| {
        is_chat(m, "event", "Toby", "queued playlist Five (1 song)")
    })
    .await;
    send(
        &mut ws,
        json!({ "type": "playlist_queue", "id": 7, "shuffle": true }),
    )
    .await;
    assert_eq!(
        wait_for(&mut ws, |m| m["type"] == "error").await["message"],
        "Queue is full"
    );
    let _ = std::fs::remove_dir_all(dir);
}

/// A YouTube import saves a playlist keyed by its list id; downloads land in YouTube order;
/// failed ones are absent; re-importing updates the same playlist (keeping a rename).
#[tokio::test]
async fn playlist_import_auto_saves_and_reimport_updates() {
    use std::os::unix::fs::PermissionsExt;
    let dir = fresh_dir("plsave");
    let music = dir.join("music");
    std::fs::create_dir_all(&music).unwrap();
    let fixture = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/flat_playlist.json"
    );
    // Downloads: hiu9WSl4OKg fails; the rest "download" at once (an empty mp3 + info JSON).
    let script = dir.join("fake-yt-dlp");
    let body = r#"#!/bin/sh
[ "$1" = --flat-playlist ] && exec cat FIXTURE
for a; do url=$a; done
id=${url##*v=}
[ "$id" = hiu9WSl4OKg ] && { echo 'ERROR: unavailable' >&2; exit 1; }
: > MUSIC/$id.mp3
echo "{\"id\":\"$id\",\"title\":\"T $id\",\"duration\":60}"
"#
    .replace("FIXTURE", fixture)
    .replace("MUSIC", &music.display().to_string());
    std::fs::write(&script, body).unwrap();
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    {
        let db = rusqlite::Connection::open(dir.join("db.sqlite")).unwrap();
        db.execute_batch(
            "CREATE TABLE tracks(id TEXT PRIMARY KEY, title TEXT NOT NULL,
                 duration_ms INTEGER NOT NULL, thumbnail INTEGER NOT NULL,
                 added_by TEXT NOT NULL, added_at INTEGER NOT NULL, gain_db REAL);
             INSERT INTO tracks VALUES ('NOaZVn2ML44', 'Good intro', 16000, 0, 'ann', 1, 0),
                                       ('TbSwCoThlbo', 'Awesome intro', 15000, 0, 'ann', 2, 0);",
        )
        .unwrap();
    }
    let (addr, dir) = start_with(dir, |c| c.ytdlp = script.display().to_string()).await;
    let (mut ws, _) = join(&addr, "Toby").await;
    let list = "PLKM3pENfZPOLCUSim2fut61AWCp9AUrQU";
    let url = format!("https://www.youtube.com/playlist?list={list}");
    send(&mut ws, json!({ "type": "add", "url": url })).await;
    let m = wait_for(&mut ws, is_playlists).await;
    let p = &m["playlists"][0];
    assert_eq!(p["name"], "10 sec Intro Music");
    assert_eq!(p["source_list_id"], list);
    assert_eq!(p["created_by"], "Toby");
    assert_eq!(
        tracks_of(&m, 0),
        ["NOaZVn2ML44", "TbSwCoThlbo"],
        "cached first"
    );
    let id = p["id"].clone();
    let done = ["NOaZVn2ML44", "9MnGSJYyR8Q", "TbSwCoThlbo", "lhYmGBmO0bs"];
    let m = wait_for(&mut ws, |m| is_playlists(m) && tracks_of(m, 0) == done).await;
    assert_eq!(m["playlists"].as_array().unwrap().len(), 1);

    // Someone renames and trims it; a re-import refreshes the songs but keeps the name and id.
    send(
        &mut ws,
        json!({ "type": "playlist_rename", "id": id, "name": "Intros" }),
    )
    .await;
    send(
        &mut ws,
        json!({ "type": "playlist_remove", "id": id, "index": 0 }),
    )
    .await;
    wait_for(&mut ws, |m| is_playlists(m) && tracks_of(m, 0).len() == 3).await;
    send(&mut ws, json!({ "type": "add", "url": url })).await;
    let m = wait_for(&mut ws, |m| is_playlists(m) && tracks_of(m, 0) == done).await;
    let all = m["playlists"].as_array().unwrap();
    assert_eq!(all.len(), 1, "no duplicate");
    assert_eq!(all[0]["id"], id);
    assert_eq!(all[0]["name"], "Intros");
    let _ = std::fs::remove_dir_all(dir);
}
