pub mod station;
pub mod ytdl;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Query, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Router, middleware};
use serde::Deserialize;
use serde_json::json;
use station::{Shared, now_ms, state_msg};
use std::path::PathBuf;
use std::sync::Arc;
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

    let media = Router::new()
        .nest_service("/audio", ServeDir::new(&cfg.music_dir))
        .nest_service("/art", ServeDir::new(&cfg.music_dir))
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

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum ClientMsg {
    Ping { t0: f64 },
    Add { url: String },
    Enqueue { track_id: String },
    Remove { queue_id: u64 },
    Move { queue_id: u64, to_index: usize },
    Play,
    Pause,
    Skip,
    Seek { position_ms: f64 },
}

async fn client(shared: Arc<Shared>, mut socket: WebSocket, name: String) {
    let mut rx = shared.tx.subscribe(); // before join, so our first broadcast is the join state
    let me = shared.join(name.clone());
    tracing::info!(id = me, %name, "listener joined");
    let welcome = json!({ "type": "welcome", "you": me }).to_string();
    if socket.send(Message::Text(welcome.into())).await.is_ok() {
        loop {
            let out = tokio::select! {
                msg = socket.recv() => match msg {
                    Some(Ok(Message::Text(t))) => handle(&shared, &t, &name),
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
    shared.leave(me);
    tracing::info!(id = me, %name, "listener left");
}

/// Handle one client frame; returns a direct reply (pong or error), if any.
fn handle(shared: &Arc<Shared>, text: &str, name: &str) -> Option<axum::extract::ws::Utf8Bytes> {
    let msg: ClientMsg = match serde_json::from_str(text) {
        Ok(m) => m,
        Err(e) => return Some(error_msg(&format!("bad message: {e}"))),
    };
    let result: Result<(), String> = match msg {
        ClientMsg::Ping { t0 } => {
            let pong = json!({ "type": "pong", "t0": t0, "server_time": now_ms() });
            return Some(pong.to_string().into());
        }
        ClientMsg::Add { url } => ytdl::handle_add(shared, &url, name),
        ClientMsg::Enqueue { track_id } => shared.enqueue(&track_id, name),
        ClientMsg::Remove { queue_id } => ok_or(
            shared.change(|st, now| st.remove(queue_id, now)),
            "No such queue item",
        ),
        ClientMsg::Move { queue_id, to_index } => ok_or(
            shared.change(|st, _| st.move_item(queue_id, to_index)),
            "No such queue item",
        ),
        ClientMsg::Play => ok_or(
            shared.change(|st, now| {
                st.play(now);
                true
            }),
            "",
        ),
        ClientMsg::Pause => ok_or(
            shared.change(|st, now| {
                st.pause(now);
                true
            }),
            "",
        ),
        ClientMsg::Skip => ok_or(
            shared.change(|st, now| {
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
    result.err().map(|e| error_msg(&e))
}

fn ok_or(ok: bool, err: &str) -> Result<(), String> {
    if ok { Ok(()) } else { Err(err.to_string()) }
}

fn error_msg(message: &str) -> axum::extract::ws::Utf8Bytes {
    json!({ "type": "error", "message": message })
        .to_string()
        .into()
}
