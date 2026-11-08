//! The one station: state, mutations, persistence, track-advance timer and broadcast.

use crate::Config;
use crate::ytdl::DlRequest;
use axum::extract::ws::Utf8Bytes;
use rusqlite::{Connection, params};
use serde::Serialize;
use serde_json::json;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::{broadcast, mpsc};

const TRACK_LEAD_MS: f64 = 1500.0;
const PLAY_LEAD_MS: f64 = 300.0;

pub fn now_ms() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
        * 1000.0
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Track {
    pub id: String,
    pub title: String,
    pub duration_ms: u64,
    pub thumbnail: bool,
    pub added_by: String,
    pub added_at: u64,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct QueueItem {
    pub queue_id: u64,
    pub track: Track,
    pub added_by: String,
}

#[derive(Serialize, Clone, Debug)]
pub struct Listener {
    pub id: u64,
    pub name: String,
}

#[derive(Serialize, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    Queued,
    Downloading,
    Done,
    Failed,
}

#[derive(Serialize, Clone, Debug)]
pub struct Job {
    pub id: u64,
    pub url: String,
    pub status: JobStatus,
    pub progress: f32,
    pub title: Option<String>,
    pub error: Option<String>,
    pub added_by: String,
}

/// Serialized as `StationState`.
#[derive(Serialize, Default)]
pub struct Station {
    pub current: Option<QueueItem>,
    pub queue: Vec<QueueItem>,
    pub playing: bool,
    pub position_ms: f64,
    pub anchor_server_ms: f64,
    pub listeners: Vec<Listener>,
    pub library: Vec<Track>,
    pub jobs: Vec<Job>,
    #[serde(skip)]
    next_id: u64,
    #[serde(skip)]
    timer_gen: u64,
}

impl Station {
    fn next_id(&mut self) -> u64 {
        self.next_id += 1;
        self.next_id
    }

    pub fn track(&self, id: &str) -> Option<&Track> {
        self.library.iter().find(|t| t.id == id)
    }

    /// Live playback position at server time `now`, clamped to the track.
    pub fn live_position(&self, now: f64) -> f64 {
        let dur = self
            .current
            .as_ref()
            .map_or(0.0, |c| c.track.duration_ms as f64);
        let pos = if self.playing {
            self.position_ms + (now - self.anchor_server_ms)
        } else {
            self.position_ms
        };
        pos.clamp(0.0, dur)
    }

    /// Server time at which the current track ends, if playing.
    pub fn end_time(&self) -> Option<f64> {
        let c = self.current.as_ref().filter(|_| self.playing)?;
        Some(self.anchor_server_ms + c.track.duration_ms as f64 - self.position_ms)
    }

    /// Make the head of the queue current (or clear), starting at 0 with the track lead time.
    pub fn advance(&mut self, now: f64, keep_playing: bool) {
        self.position_ms = 0.0;
        if self.queue.is_empty() {
            self.current = None;
            self.playing = false;
            self.anchor_server_ms = now;
        } else {
            self.current = Some(self.queue.remove(0));
            self.playing = keep_playing;
            self.anchor_server_ms = if keep_playing {
                now + TRACK_LEAD_MS
            } else {
                now
            };
        }
    }

    pub fn play(&mut self, now: f64) {
        if self.current.is_none() {
            self.advance(now, true);
        } else if !self.playing {
            self.playing = true;
            self.anchor_server_ms = now + PLAY_LEAD_MS;
        }
    }

    pub fn pause(&mut self, now: f64) {
        if self.playing {
            self.position_ms = self.live_position(now);
            self.playing = false;
            self.anchor_server_ms = now;
        }
    }

    pub fn seek(&mut self, now: f64, pos: f64) {
        if self.current.is_none() || !pos.is_finite() {
            return;
        }
        let dur = self
            .current
            .as_ref()
            .map_or(0.0, |c| c.track.duration_ms as f64);
        self.position_ms = pos.clamp(0.0, dur);
        self.anchor_server_ms = if self.playing {
            now + PLAY_LEAD_MS
        } else {
            now
        };
    }

    /// Append to the queue; an idle station starts playing it.
    pub fn push(&mut self, track: Track, added_by: &str, now: f64) {
        let item = QueueItem {
            queue_id: self.next_id(),
            track,
            added_by: added_by.to_string(),
        };
        self.queue.push(item);
        if self.current.is_none() {
            self.advance(now, true);
        }
    }

    pub fn remove(&mut self, queue_id: u64, now: f64) -> bool {
        if self
            .current
            .as_ref()
            .is_some_and(|c| c.queue_id == queue_id)
        {
            let playing = self.playing;
            self.advance(now, playing);
            return true;
        }
        let before = self.queue.len();
        self.queue.retain(|q| q.queue_id != queue_id);
        self.queue.len() != before
    }

    pub fn move_item(&mut self, queue_id: u64, to_index: usize) -> bool {
        let Some(from) = self.queue.iter().position(|q| q.queue_id == queue_id) else {
            return false;
        };
        let item = self.queue.remove(from);
        let to = to_index.min(self.queue.len());
        self.queue.insert(to, item);
        true
    }
}

pub struct Shared {
    station: Mutex<Station>,
    db: Mutex<Connection>,
    pub tx: broadcast::Sender<Utf8Bytes>,
    pub dl_tx: mpsc::UnboundedSender<DlRequest>,
    pub cfg: Config,
}

pub fn state_msg(st: &Station) -> Utf8Bytes {
    json!({ "type": "state", "state": st }).to_string().into()
}

impl Shared {
    pub fn new(cfg: Config) -> rusqlite::Result<(Arc<Self>, mpsc::UnboundedReceiver<DlRequest>)> {
        let db = Connection::open(&cfg.db_path)?;
        let station = load(&db)?;
        let (tx, _) = broadcast::channel(64);
        let (dl_tx, dl_rx) = mpsc::unbounded_channel();
        let shared = Arc::new(Self {
            station: Mutex::new(station),
            db: Mutex::new(db),
            tx,
            dl_tx,
            cfg,
        });
        Ok((shared, dl_rx))
    }

    pub fn lock(&self) -> MutexGuard<'_, Station> {
        self.station.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn send(&self, msg: Utf8Bytes) {
        let _ = self.tx.send(msg); // Err only means no listeners
    }

    /// Mutate the station; if `f` returns true: persist, reschedule the timer, broadcast state.
    pub fn change(self: &Arc<Self>, f: impl FnOnce(&mut Station, f64) -> bool) -> bool {
        let now = now_ms();
        let mut st = self.lock();
        if !f(&mut st, now) {
            return false;
        }
        self.persist(&st, now);
        self.schedule(&mut st, now);
        self.send(state_msg(&st));
        true
    }

    /// Broadcast state without persisting (listeners and jobs are not stored).
    fn broadcast_state(&self, st: &Station) {
        self.send(state_msg(st));
    }

    fn schedule(self: &Arc<Self>, st: &mut Station, now: f64) {
        st.timer_gen += 1;
        let Some(end) = st.end_time() else { return };
        let generation = st.timer_gen;
        let me = Arc::clone(self);
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs_f64(((end - now) / 1000.0).max(0.0))).await;
            me.change(|st, now| {
                if st.timer_gen != generation {
                    return false; // cancelled by a later change
                }
                st.advance(now, true);
                true
            });
        });
    }

    pub fn join(&self, name: String) -> u64 {
        let mut st = self.lock();
        let id = st.next_id();
        st.listeners.push(Listener { id, name });
        self.broadcast_state(&st);
        id
    }

    pub fn leave(&self, id: u64) {
        let mut st = self.lock();
        st.listeners.retain(|l| l.id != id);
        self.broadcast_state(&st);
    }

    pub fn enqueue(self: &Arc<Self>, track_id: &str, added_by: &str) -> Result<(), String> {
        let mut ok = false;
        self.change(|st, now| {
            if let Some(t) = st.track(track_id).cloned() {
                st.push(t, added_by, now);
                ok = true;
            }
            ok
        });
        ok.then_some(()).ok_or_else(|| "Unknown track".into())
    }

    pub fn add_job(&self, mut job: Job) -> u64 {
        let mut st = self.lock();
        job.id = st.next_id();
        let id = job.id;
        self.send(json!({ "type": "job", "job": job }).to_string().into());
        st.jobs.push(job);
        self.broadcast_state(&st);
        id
    }

    /// Update a job in place and send a `job` message (no full state broadcast).
    pub fn update_job(&self, id: u64, f: impl FnOnce(&mut Job)) {
        let mut st = self.lock();
        if let Some(j) = st.jobs.iter_mut().find(|j| j.id == id) {
            f(j);
            self.send(json!({ "type": "job", "job": j }).to_string().into());
        }
    }

    pub fn finish_job(self: &Arc<Self>, id: u64, mut track: Track) {
        let added_by = {
            let st = self.lock();
            st.jobs
                .iter()
                .find(|j| j.id == id)
                .map(|j| j.added_by.clone())
                .unwrap_or_default()
        };
        self.update_job(id, |j| {
            j.status = JobStatus::Done;
            j.progress = 1.0;
            j.title = Some(track.title.clone());
        });
        self.change(|st, now| {
            st.jobs.retain(|j| j.id != id);
            if st.track(&track.id).is_none() {
                track.added_by.clone_from(&added_by);
                self.insert_track(&track);
                st.library.insert(0, track.clone());
            }
            st.push(track, &added_by, now);
            true
        });
    }

    pub fn fail_job(self: &Arc<Self>, id: u64, error: String) {
        self.update_job(id, |j| {
            j.status = JobStatus::Failed;
            j.error = Some(error);
        });
        self.broadcast_state(&self.lock());
        let me = Arc::clone(self);
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(300)).await;
            let mut st = me.lock();
            st.jobs.retain(|j| j.id != id);
            me.broadcast_state(&st);
        });
    }

    fn db(&self) -> MutexGuard<'_, Connection> {
        self.db.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn insert_track(&self, t: &Track) {
        let r = self.db().execute(
            "INSERT OR REPLACE INTO tracks VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                t.id,
                t.title,
                t.duration_ms,
                t.thumbnail,
                t.added_by,
                t.added_at
            ],
        );
        if let Err(e) = r {
            tracing::error!("saving track: {e}");
        }
    }

    /// Save queue + current item + live position (tiny, so rewrite everything).
    pub fn persist(&self, st: &Station, now: f64) {
        let mut db = self.db();
        let r = (|| {
            let tx = db.transaction()?;
            tx.execute("DELETE FROM queue", [])?;
            for (i, q) in st.queue.iter().enumerate() {
                tx.execute(
                    "INSERT INTO queue VALUES (?1, ?2, ?3, ?4)",
                    params![q.queue_id, q.track.id, q.added_by, i as i64],
                )?;
            }
            let current = st.current.as_ref().map(|c| {
                json!({ "queue_id": c.queue_id, "track_id": c.track.id, "added_by": c.added_by })
                    .to_string()
            });
            let kv = [
                ("current", current.unwrap_or_default()),
                ("position_ms", st.live_position(now).to_string()),
                ("next_id", st.next_id.to_string()),
            ];
            for (k, v) in kv {
                tx.execute(
                    "INSERT OR REPLACE INTO station VALUES (?1, ?2)",
                    params![k, v],
                )?;
            }
            tx.commit()
        })();
        if let Err(e) = r {
            tracing::error!("persisting station: {e}");
        }
    }
}

fn load(db: &Connection) -> rusqlite::Result<Station> {
    db.execute_batch(
        "PRAGMA journal_mode=WAL;
         CREATE TABLE IF NOT EXISTS tracks(id TEXT PRIMARY KEY, title TEXT NOT NULL,
             duration_ms INTEGER NOT NULL, thumbnail INTEGER NOT NULL,
             added_by TEXT NOT NULL, added_at INTEGER NOT NULL);
         CREATE TABLE IF NOT EXISTS queue(queue_id INTEGER PRIMARY KEY, track_id TEXT NOT NULL,
             added_by TEXT NOT NULL, position INTEGER NOT NULL);
         CREATE TABLE IF NOT EXISTS station(k TEXT PRIMARY KEY, v TEXT NOT NULL);",
    )?;
    let library = db
        .prepare("SELECT id, title, duration_ms, thumbnail, added_by, added_at FROM tracks ORDER BY added_at DESC")?
        .query_map([], |r| {
            Ok(Track {
                id: r.get(0)?,
                title: r.get(1)?,
                duration_ms: r.get(2)?,
                thumbnail: r.get(3)?,
                added_by: r.get(4)?,
                added_at: r.get(5)?,
            })
        })?
        .collect::<rusqlite::Result<_>>()?;
    let mut st = Station {
        library,
        ..Station::default()
    };
    let item = |st: &Station, queue_id: u64, track_id: &str, added_by: String| {
        st.track(track_id).map(|t| QueueItem {
            queue_id,
            track: t.clone(),
            added_by,
        })
    };
    let rows: Vec<(u64, String, String)> = db
        .prepare("SELECT queue_id, track_id, added_by FROM queue ORDER BY position")?
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
        .collect::<rusqlite::Result<_>>()?;
    st.queue = rows
        .into_iter()
        .filter_map(|(q, t, a)| item(&st, q, &t, a))
        .collect();
    let kv = |k: &str| -> Option<String> {
        db.query_row("SELECT v FROM station WHERE k = ?1", [k], |r| r.get(0))
            .ok()
    };
    if let Some(c) = kv("current").and_then(|v| serde_json::from_str::<serde_json::Value>(&v).ok())
    {
        st.current = item(
            &st,
            c["queue_id"].as_u64().unwrap_or(0),
            c["track_id"].as_str().unwrap_or(""),
            c["added_by"].as_str().unwrap_or("").to_string(),
        );
    }
    // Restart resumes paused at the saved position.
    st.position_ms = kv("position_ms")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0.0);
    st.position_ms = st.live_position(0.0);
    st.anchor_server_ms = now_ms();
    st.next_id = kv("next_id").and_then(|v| v.parse().ok()).unwrap_or(0);
    let max_q = st
        .queue
        .iter()
        .chain(&st.current)
        .map(|q| q.queue_id)
        .max()
        .unwrap_or(0);
    st.next_id = st.next_id.max(max_q);
    Ok(st)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn track(id: &str, dur: u64) -> Track {
        Track {
            id: id.into(),
            title: id.into(),
            duration_ms: dur,
            thumbnail: false,
            added_by: "t".into(),
            added_at: 0,
        }
    }

    fn station(ids: &[&str]) -> Station {
        let mut st = Station::default();
        for id in ids {
            let item = QueueItem {
                queue_id: st.next_id(),
                track: track(id, 10_000),
                added_by: "t".into(),
            };
            st.queue.push(item);
        }
        st
    }

    fn order(st: &Station) -> Vec<&str> {
        st.queue.iter().map(|q| q.track.id.as_str()).collect()
    }

    #[test]
    fn position_math() {
        let mut st = station(&["a", "b"]);
        st.play(1000.0); // starts head with track lead
        assert_eq!(st.current.as_ref().unwrap().track.id, "a");
        assert_eq!(st.anchor_server_ms, 2500.0);
        assert_eq!(st.live_position(2000.0), 0.0); // before anchor clamps to 0
        assert_eq!(st.live_position(5500.0), 3000.0);
        assert_eq!(st.end_time(), Some(12_500.0));
        assert_eq!(st.live_position(99_999.0), 10_000.0); // clamped to duration

        st.pause(5500.0);
        assert!(!st.playing);
        assert_eq!(st.position_ms, 3000.0);
        assert_eq!(st.live_position(9999.0), 3000.0);
        assert_eq!(st.end_time(), None);

        st.play(6000.0);
        assert_eq!(st.anchor_server_ms, 6300.0);
        assert_eq!(st.end_time(), Some(6300.0 + 7000.0));

        st.seek(7000.0, 8000.0);
        assert_eq!(st.position_ms, 8000.0);
        assert_eq!(st.anchor_server_ms, 7300.0);
        st.seek(7000.0, 1e9);
        assert_eq!(st.position_ms, 10_000.0);

        st.advance(20_000.0, true);
        assert_eq!(st.current.as_ref().unwrap().track.id, "b");
        assert_eq!((st.position_ms, st.anchor_server_ms), (0.0, 21_500.0));
        st.advance(40_000.0, true);
        assert!(st.current.is_none() && !st.playing);
    }

    #[test]
    fn push_into_idle_station_starts_playing() {
        let mut st = Station::default();
        st.push(track("a", 1000), "x", 0.0);
        assert!(st.playing);
        assert_eq!(st.current.as_ref().unwrap().added_by, "x");
        st.push(track("b", 1000), "y", 0.0);
        assert_eq!(order(&st), ["b"]);
    }

    #[test]
    fn queue_move_remove() {
        let mut st = station(&["a", "b", "c", "d"]); // queue ids 1..=4
        assert!(st.move_item(1, 2));
        assert_eq!(order(&st), ["b", "c", "a", "d"]);
        assert!(st.move_item(4, 0));
        assert_eq!(order(&st), ["d", "b", "c", "a"]);
        assert!(st.move_item(2, 99));
        assert_eq!(order(&st), ["d", "c", "a", "b"]);
        assert!(!st.move_item(42, 0));
        assert!(st.remove(3, 0.0));
        assert_eq!(order(&st), ["d", "a", "b"]);
        assert!(!st.remove(3, 0.0));
        st.play(0.0); // d becomes current
        assert!(st.remove(4, 0.0)); // removing current skips
        assert_eq!(st.current.as_ref().unwrap().track.id, "a");
        assert_eq!(order(&st), ["b"]);
    }

    #[test]
    fn persistence_roundtrip_resumes_paused() {
        let dir = std::env::temp_dir().join(format!("cb-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let cfg = Config {
            db_path: dir.join("t.db"),
            ..Config::from_env()
        };
        let _ = std::fs::remove_file(&cfg.db_path);
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            let (shared, _rx) = Shared::new(cfg.clone()).unwrap();
            for id in ["a", "b", "c"] {
                let t = track(id, 60_000);
                shared.insert_track(&t);
                shared.lock().library.push(t);
            }
            for id in ["a", "b", "c"] {
                shared.enqueue(id, "toby").unwrap();
            }
            shared.change(|st, now| {
                st.pause(now);
                st.seek(now, 5000.0);
                true
            });
            drop(shared);
            let (shared, _rx) = Shared::new(cfg).unwrap();
            let st = shared.lock();
            assert!(!st.playing);
            assert_eq!(st.current.as_ref().unwrap().track.id, "a");
            assert_eq!(st.position_ms, 5000.0);
            assert_eq!(order(&st), ["b", "c"]);
            assert_eq!(st.next_id, 3);
        });
        let _ = std::fs::remove_dir_all(dir);
    }
}
