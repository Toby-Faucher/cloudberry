pub mod loudness;
pub mod station;
pub mod ytdl;

use axum::body::Body;
use axum::extract::ws::{Message, Utf8Bytes, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, Query, Request, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Router, middleware};
use serde::Deserialize;
use serde_json::json;
use station::{Shared, now_ms, state_msg};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::broadcast::error::RecvError;
use tower_http::services::{ServeDir, ServeFile};

#[derive(Clone, Debug)]
pub struct Config {
    pub music_dir: PathBuf,
    pub db_path: PathBuf,
    pub bind: String,
    pub static_dir: PathBuf,
    pub ytdlp: String,
}

impl Config {
    pub fn from_env() -> Self {
        let var = |k: &str, d: &str| std::env::var(k).unwrap_or_else(|_| d.to_string());
        Self {
            music_dir: var("CLOUDBERRY_MUSIC_DIR", "./data/music").into(),
            db_path: var("CLOUDBERRY_DB_PATH", "./data/cloudberry.db").into(),
            bind: var("CLOUDBERRY_BIND", "0.0.0.0:8080"),
            static_dir: var("CLOUDBERRY_STATIC_DIR", "./web/build").into(),
            ytdlp: var("CLOUDBERRY_YTDLP", "yt-dlp"),
        }
    }
}

/// Build shared state, start the download worker and return the router.
pub fn app(cfg: Config) -> Result<(Router, Arc<Shared>), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(&cfg.music_dir)?;
    if let Some(p) = cfg.db_path.parent() {
        std::fs::create_dir_all(p)?;
    }
    let (shared, dl_rx) = Shared::new(cfg.clone())?;
    tokio::spawn(ytdl::worker(Arc::clone(&shared), dl_rx));
    tokio::spawn(loudness::backfill(Arc::clone(&shared)));
    tokio::spawn(Arc::clone(&shared).dj_rollover());

    let media = Router::new()
        .route("/audio/{file}", get(audio))
        .route("/art/{file}", get(art))
        .layer(middleware::map_response(|mut res: Response| async move {
            if res.status().is_success() {
                res.headers_mut().insert(
                    header::CACHE_CONTROL,
                    HeaderValue::from_static("public, max-age=31536000, immutable"),
                );
            }
            res
        }));
    let site =
        ServeDir::new(&cfg.static_dir).fallback(ServeFile::new(cfg.static_dir.join("index.html")));
    let router = Router::new()
        .route("/api/state", get(api_state))
        .route("/ws", get(ws_upgrade))
        .merge(media)
        .fallback_service(site)
        .with_state(Arc::clone(&shared));
    Ok((router, shared))
}

async fn audio(State(s): State<Arc<Shared>>, Path(f): Path<String>, req: Request) -> Response {
    media_file(&s, &f, ".mp3", req).await
}

async fn art(State(s): State<Arc<Shared>>, Path(f): Path<String>, req: Request) -> Response {
    media_file(&s, &f, ".jpg", req).await
}

/// Serve `{11-char id}{ext}` only, so yt-dlp temp/partial files are never reachable.
async fn media_file(shared: &Shared, file: &str, ext: &str, req: Request) -> Response {
    if !file.strip_suffix(ext).is_some_and(ytdl::is_video_id) {
        return StatusCode::NOT_FOUND.into_response();
    }
    match ServeFile::new(shared.cfg.music_dir.join(file))
        .try_call(req)
        .await
    {
        Ok(res) => res.map(Body::new),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

async fn api_state(State(shared): State<Arc<Shared>>) -> Response {
    // Serialize under the lock, respond after releasing it.
    let body = serde_json::to_vec(&*shared.lock()).unwrap_or_default();
    ([(header::CONTENT_TYPE, "application/json")], body).into_response()
}

#[derive(Deserialize)]
struct WsQuery {
    name: Option<String>,
}

async fn ws_upgrade(
    State(shared): State<Arc<Shared>>,
    Query(q): Query<WsQuery>,
    ws: WebSocketUpgrade,
) -> Response {
    let name = q.name.as_deref().unwrap_or("").trim().to_string();
    if !(1..=32).contains(&name.chars().count()) {
        return (StatusCode::BAD_REQUEST, "name must be 1-32 characters").into_response();
    }
    ws.max_message_size(64 * 1024)
        .on_upgrade(move |socket| client(shared, socket, name))
}

const REACTIONS: [&str; 6] = ["🔥", "😂", "❤️", "👏", "🎉", "😴"];

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum ClientMsg {
    Ping { t0: f64 },
    Add { url: String },
    Enqueue { track_id: String },
    Insert { track_id: String, index: usize },
    Remove { queue_id: u64 },
    Move { queue_id: u64, to_index: usize },
    Play,
    Pause,
    Skip,
    Seek { position_ms: f64 },
    Chat { text: String },
    React { emoji: String },
    Poke { listener_id: u64 },
    Presence { away: bool },
}

/// Sliding-window rate limit: at most `max` hits per 5 s.
struct Limiter {
    hits: VecDeque<Instant>,
    max: usize,
}

impl Limiter {
    fn new(max: usize) -> Self {
        Self {
            hits: VecDeque::with_capacity(max),
            max,
        }
    }

    fn allow(&mut self) -> bool {
        let now = Instant::now();
        while self
            .hits
            .front()
            .is_some_and(|t| now - *t >= Duration::from_secs(5))
        {
            self.hits.pop_front();
        }
        let ok = self.hits.len() < self.max;
        if ok {
            self.hits.push_back(now);
        }
        ok
    }
}

struct Conn {
    id: u64,
    name: String,
    chat: Limiter,
    react: Limiter,
    last_poke: Option<Instant>,
}

async fn client(shared: Arc<Shared>, mut socket: WebSocket, name: String) {
    // Subscribe before join so our first broadcasts are the join state and "joined" event;
    // history is read atomically with subscribing, so nothing is delivered twice.
    let (mut rx, history) = shared.subscribe_with_history();
    let me = shared.join(name.clone());
    shared.event(&name, "joined");
    tracing::info!(id = me, %name, "listener joined");
    let mut conn = Conn {
        id: me,
        name,
        chat: Limiter::new(5),
        react: Limiter::new(10),
        last_poke: None,
    };
    let welcome = json!({ "type": "welcome", "you": me }).to_string();
    let history = json!({ "type": "chat_history", "messages": history }).to_string();
    if socket.send(Message::Text(welcome.into())).await.is_ok()
        && socket.send(Message::Text(history.into())).await.is_ok()
    {
        loop {
            let out = tokio::select! {
                msg = socket.recv() => match msg {
                    Some(Ok(Message::Text(t))) => handle(&shared, &mut conn, &t),
                    Some(Ok(_)) => None,
                    _ => break,
                },
                b = rx.recv() => match b {
                    Ok(m) => Some(m),
                    // Fell behind: the latest state supersedes what we missed.
                    Err(RecvError::Lagged(_)) => Some(state_msg(&shared.lock())),
                    Err(RecvError::Closed) => break,
                },
            };
            if let Some(m) = out
                && socket.send(Message::Text(m)).await.is_err()
            {
                break;
            }
        }
    }
    let paused = shared.leave(me);
    shared.event(&conn.name, "left");
    if paused {
        shared.event("Cloudberry", "paused");
    }
    tracing::info!(id = me, name = %conn.name, "listener left");
}

/// Handle one client frame; returns a direct reply (pong or error), if any.
fn handle(shared: &Arc<Shared>, conn: &mut Conn, text: &str) -> Option<Utf8Bytes> {
    let msg: ClientMsg = match serde_json::from_str(text) {
        Ok(m) => m,
        Err(e) => return Some(error_msg(&format!("bad message: {e}"))),
    };
    let name = conn.name.as_str();
    let mut event: Option<String> = None; // posted to chat if the action succeeded
    let result: Result<(), String> = match msg {
        ClientMsg::Ping { t0 } => {
            let pong = json!({ "type": "pong", "t0": t0, "server_time": now_ms() });
            return Some(pong.to_string().into());
        }
        ClientMsg::Chat { text } => {
            let text = text.trim();
            if !(1..=500).contains(&text.chars().count()) {
                return Some(error_msg("Chat messages must be 1-500 characters"));
            }
            if conn.chat.allow() {
                shared.post_chat("user", name, text);
            }
            return None;
        }
        ClientMsg::React { emoji } => {
            if !REACTIONS.contains(&emoji.as_str()) {
                return Some(error_msg("Unsupported reaction"));
            }
            if conn.react.allow() {
                let m = json!({ "type": "react", "listener_id": conn.id, "name": name, "emoji": emoji });
                let _ = shared.tx.send(m.to_string().into());
                shared.count_reaction();
            }
            return None;
        }
        ClientMsg::Poke { listener_id } => {
            let known = shared.lock().listeners.iter().any(|l| l.id == listener_id);
            if listener_id == conn.id || !known {
                return Some(error_msg("Can't poke that listener"));
            }
            if conn
                .last_poke
                .is_none_or(|t| t.elapsed() >= Duration::from_secs(2))
            {
                conn.last_poke = Some(Instant::now());
                let m = json!({ "type": "poke", "from_id": conn.id, "from_name": name, "to_id": listener_id });
                let _ = shared.tx.send(m.to_string().into());
            }
            return None;
        }
        ClientMsg::Presence { away } => {
            shared.set_away(conn.id, away);
            return None;
        }
        ClientMsg::Add { url } => ytdl::handle_add(shared, &url, name),
        ClientMsg::Enqueue { track_id } => shared.enqueue(&track_id, name, usize::MAX),
        ClientMsg::Insert { track_id, index } => shared.enqueue(&track_id, name, index),
        ClientMsg::Remove { queue_id } => ok_or(
            shared.change(|st, now| {
                event = st
                    .queue
                    .iter()
                    .chain(&st.current)
                    .find(|q| q.queue_id == queue_id)
                    .map(|q| format!("removed {}", q.track.title));
                st.remove(queue_id, now)
            }),
            "No such queue item",
        ),
        ClientMsg::Move { queue_id, to_index } => ok_or(
            shared.change(|st, _| st.move_item(queue_id, to_index)),
            "No such queue item",
        ),
        ClientMsg::Play => ok_or(
            shared.change(|st, now| {
                let was = st.playing;
                st.play(now);
                if !was && st.playing {
                    event = Some("resumed".into());
                }
                true
            }),
            "",
        ),
        ClientMsg::Pause => ok_or(
            shared.change(|st, now| {
                if st.playing {
                    event = Some("paused".into());
                }
                st.pause(now);
                true
            }),
            "",
        ),
        ClientMsg::Skip => ok_or(
            shared.change(|st, now| {
                event = st
                    .current
                    .as_ref()
                    .map(|c| format!("skipped {}", c.track.title));
                let playing = st.playing;
                st.advance(now, playing);
                true
            }),
            "",
        ),
        ClientMsg::Seek { position_ms } => ok_or(
            shared.change(|st, now| {
                st.seek(now, position_ms);
                st.current.is_some()
            }),
            "Nothing is playing",
        ),
    };
    if let Some(e) = event {
        shared.event(name, &e);
    }
    result.err().map(|e| error_msg(&e))
}

fn ok_or(ok: bool, err: &str) -> Result<(), String> {
    if ok { Ok(()) } else { Err(err.to_string()) }
}

fn error_msg(message: &str) -> Utf8Bytes {
    json!({ "type": "error", "message": message })
        .to_string()
        .into()
}
