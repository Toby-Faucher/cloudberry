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
const HISTORY_LEN: usize = 20;
const CHAT_KEEP: i64 = 500;
const CHAT_HISTORY: i64 = 100;
pub const MAX_QUEUE: usize = 200;
pub const QUEUE_FULL: &str = "Queue is full";
pub const TOO_MANY_JOBS: &str = "Too many downloads queued, try again soon";
/// Pending (queued + downloading) jobs in total, so a 50-song playlist fits...
const MAX_PENDING_JOBS: usize = 60;
/// ...while single-video adds stop at 10 pending single jobs.
const MAX_PENDING_SINGLES: usize = 10;
/// How long a finished playlist's job rows stay visible.
const PLAYLIST_LINGER: Duration = Duration::from_secs(8);

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
    /// Loudness normalisation for clients' GainNode; 0 until measured.
    pub gain_db: f32,
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
    pub away: bool,
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
    /// Set for jobs created by a playlist import, so clients can group them.
    pub playlist: Option<PlaylistRef>,
    /// Import jobs: the playlist's video ids in YouTube order and this job's index in them, so
    /// the finished track lands at the right spot in the saved playlist.
    #[serde(skip)]
    pub import_pos: Option<(Arc<[String]>, usize)>,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct PlaylistRef {
    pub id: String,
    pub title: String,
}

impl Job {
    fn pending(&self) -> bool {
        matches!(self.status, JobStatus::Queued | JobStatus::Downloading)
    }

    fn in_playlist(&self, id: &str) -> bool {
        self.playlist.as_ref().is_some_and(|p| p.id == id)
    }
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
    /// Newest first. Sent as its own `library` message (it's the big part), not in `state`.
    #[serde(skip)]
    pub library: Vec<Track>,
    pub jobs: Vec<Job>,
    /// Last played tracks (left `current` by ending, skip or remove), newest first.
    pub history: Vec<Track>,
    /// Whose queued tracks got the most reactions today (server local day).
    pub dj_of_day: Option<String>,
    /// Saved shared playlists, most recently updated first. Sent as its own `playlists` message.
    #[serde(skip)]
    pub playlists: Vec<crate::playlists::Playlist>,
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

    /// Append to the queue; an idle station starts playing it. False if the queue is full.
    pub fn push(&mut self, track: Track, added_by: &str, now: f64) -> bool {
        self.insert(track, added_by, usize::MAX, now)
    }

    /// Insert into the queue at `index` (clamped); an idle station starts playing it.
    /// False (and nothing inserted) if the queue already holds `MAX_QUEUE` items.
    pub fn insert(&mut self, track: Track, added_by: &str, index: usize, now: f64) -> bool {
        if self.queue.len() >= MAX_QUEUE {
            return false;
        }
        let item = QueueItem {
            queue_id: self.next_id(),
            track,
            added_by: added_by.to_string(),
        };
        self.queue.insert(index.min(self.queue.len()), item);
        if self.current.is_none() {
            self.advance(now, true);
        }
        true
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

    pub fn set_gain(&mut self, id: &str, gain_db: f32) {
        let items = self.queue.iter_mut().chain(&mut self.current);
        let tracks = self.library.iter_mut().chain(&mut self.history);
        for t in tracks.chain(items.map(|q| &mut q.track)) {
            if t.id == id {
                t.gain_db = gain_db;
            }
        }
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
    /// One playlist expansion (a yt-dlp process) at a time.
    pub expanding: tokio::sync::Semaphore,
}

#[derive(Serialize, Clone, Debug)]
pub struct ChatMessage {
    pub id: u64,
    /// "user" or "event"
    pub kind: &'static str,
    pub name: String,
    pub text: String,
    pub at: u64,
}

pub fn state_msg(st: &Station) -> Utf8Bytes {
    json!({ "type": "state", "state": st }).to_string().into()
}

pub fn library_msg(st: &Station) -> Utf8Bytes {
    json!({ "type": "library", "tracks": st.library })
        .to_string()
        .into()
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
            expanding: tokio::sync::Semaphore::new(1),
        });
        Ok((shared, dl_rx))
    }

    pub fn lock(&self) -> MutexGuard<'_, Station> {
        self.station.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub(crate) fn send(&self, msg: Utf8Bytes) {
        let _ = self.tx.send(msg); // Err only means no listeners
    }

    /// Mutate the station; if `f` returns true: persist, reschedule the timer, broadcast state.
    pub fn change(self: &Arc<Self>, f: impl FnOnce(&mut Station, f64) -> bool) -> bool {
        let now = now_ms();
        let mut st = self.lock();
        let before = st.current.as_ref().map(|c| (c.queue_id, c.track.clone()));
        if !f(&mut st, now) {
            return false;
        }
        if let Some((qid, track)) = before
            && st.current.as_ref().map(|c| c.queue_id) != Some(qid)
        {
            self.record_played(&mut st, track, now);
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
        st.listeners.push(Listener {
            id,
            name,
            away: false,
        });
        self.broadcast_state(&st);
        id
    }

    /// Remove a listener; the last one leaving pauses the station. Returns whether it paused.
    pub fn leave(self: &Arc<Self>, id: u64) -> bool {
        let mut paused = false;
        self.change(|st, now| {
            st.listeners.retain(|l| l.id != id);
            if st.listeners.is_empty() && st.playing {
                st.pause(now);
                paused = true;
            }
            true
        });
        paused
    }

    pub fn set_away(&self, id: u64, away: bool) {
        let mut st = self.lock();
        if let Some(l) = st.listeners.iter_mut().find(|l| l.id == id) {
            l.away = away;
        }
        self.broadcast_state(&st);
    }

    /// Credit a reaction to whoever queued the current track (ignored when nothing is current).
    pub fn count_reaction(&self) {
        let mut st = self.lock();
        let Some(dj) = st.current.as_ref().map(|c| c.added_by.clone()) else {
            return;
        };
        let db = self.db();
        let r = db.execute(
            "INSERT INTO reactions VALUES (date('now', 'localtime'), ?1, 1, ?2)
                 ON CONFLICT(day, name) DO UPDATE SET count = count + 1, last_at = excluded.last_at",
            params![dj, now_ms() as u64],
        );
        if let Err(e) = r {
            tracing::error!("saving reaction: {e}");
        }
        let top = dj_of_day(&db);
        drop(db);
        if top != st.dj_of_day {
            st.dj_of_day = top;
            self.broadcast_state(&st);
        }
    }

    /// Re-read today's DJ just after each local midnight.
    pub async fn dj_rollover(self: Arc<Self>) {
        loop {
            let secs = secs_to_local_midnight(&self.db());
            tokio::time::sleep(Duration::from_secs_f64(secs.max(0.0) + 1.0)).await;
            let mut st = self.lock();
            let top = dj_of_day(&self.db());
            if top != st.dj_of_day {
                st.dj_of_day = top;
                self.broadcast_state(&st);
            }
        }
    }

    /// Queue a library track at `index` (clamped; `usize::MAX` appends) and post an event.
    pub fn enqueue(
        self: &Arc<Self>,
        track_id: &str,
        added_by: &str,
        index: usize,
    ) -> Result<(), String> {
        let mut title = None;
        let queued = self.change(|st, now| {
            let Some(t) = st.track(track_id).cloned() else {
                return false;
            };
            title = Some(t.title.clone());
            st.insert(t, added_by, index, now)
        });
        let title = title.ok_or("Unknown track")?;
        if !queued {
            return Err(QUEUE_FULL.into());
        }
        self.event(added_by, &format!("added {title}"));
        Ok(())
    }

    /// Add a queued job (clients see it via the state broadcast), unless too many are pending.
    pub fn add_job(&self, mut job: Job) -> Result<u64, String> {
        let mut st = self.lock();
        let pending = st.jobs.iter().filter(|j| j.pending());
        let singles = pending.clone().filter(|j| j.playlist.is_none()).count();
        if pending.count() >= MAX_PENDING_JOBS || singles >= MAX_PENDING_SINGLES {
            return Err(TOO_MANY_JOBS.into());
        }
        job.id = st.next_id();
        let id = job.id;
        st.jobs.push(job);
        self.broadcast_state(&st);
        Ok(id)
    }

    /// Enqueue a playlist's cached tracks now and create download jobs for the rest, as far as
    /// pending-job capacity allows. Returns (songs added, skipped incl. `pl.skipped`, downloads).
    pub fn import_playlist(
        self: &Arc<Self>,
        list_id: &str,
        pl: &crate::ytdl::Playlist,
        added_by: &str,
    ) -> Result<(usize, usize, Vec<DlRequest>), String> {
        let mut missing = Vec::new();
        let (mut cached, mut full) = (0, false);
        let order: Arc<[String]> = pl.entries.iter().map(|(id, _)| id.clone()).collect();
        self.change(|st, now| {
            for (k, (id, title)) in pl.entries.iter().enumerate() {
                match st.track(id).cloned() {
                    Some(t) => {
                        if st.push(t, added_by, now) {
                            cached += 1;
                        } else {
                            full = true;
                        }
                    }
                    None => missing.push((k, id, title)),
                }
            }
            cached > 0
        });
        let mut st = self.lock();
        let pending = st.jobs.iter().filter(|j| j.pending()).count();
        let room = MAX_PENDING_JOBS.saturating_sub(pending);
        let mut requests = Vec::new();
        for (k, id, title) in missing.iter().take(room) {
            let job_id = st.next_id();
            st.jobs.push(Job {
                id: job_id,
                url: crate::ytdl::canonical_url(id),
                status: JobStatus::Queued,
                progress: 0.0,
                title: title.as_ref().map(|t| t.to_string()),
                error: None,
                added_by: added_by.to_string(),
                playlist: Some(PlaylistRef {
                    id: list_id.to_string(),
                    title: pl.title.clone(),
                }),
                import_pos: Some((Arc::clone(&order), *k)),
            });
            requests.push(DlRequest {
                job_id,
                video_id: id.to_string(),
            });
        }
        if !requests.is_empty() {
            self.broadcast_state(&st);
        }
        let songs = cached + requests.len();
        let skipped = pl.entries.len() - songs + pl.skipped;
        if songs == 0 {
            return Err(if full {
                QUEUE_FULL.into()
            } else if missing.len() > room {
                TOO_MANY_JOBS.into()
            } else {
                "Nothing in that playlist can be added".into()
            });
        }
        Ok((songs, skipped, requests))
    }

    /// Drop a playlist's not-yet-started jobs (a running download finishes). Returns its title.
    pub fn cancel_playlist(self: &Arc<Self>, list_id: &str) -> Option<String> {
        let mut st = self.lock();
        let title = st
            .jobs
            .iter()
            .find(|j| j.status == JobStatus::Queued && j.in_playlist(list_id))
            .and_then(|j| j.playlist.as_ref())
            .map(|p| p.title.clone())?;
        st.jobs
            .retain(|j| !(j.status == JobStatus::Queued && j.in_playlist(list_id)));
        self.broadcast_state(&st);
        drop(st);
        self.reap_playlist(list_id);
        Some(title)
    }

    /// Once nothing of a playlist is pending, clear all its rows after a short linger.
    fn reap_playlist(self: &Arc<Self>, list_id: &str) {
        let st = self.lock();
        let jobs = st.jobs.iter().filter(|j| j.in_playlist(list_id));
        if jobs.clone().any(Job::pending) {
            return;
        }
        let ids = jobs.map(|j| j.id).collect();
        drop(st);
        self.remove_jobs_after(ids, PLAYLIST_LINGER);
    }

    /// Schedule a finished job's removal: singles on their own, playlist jobs as a group.
    fn retire_job(self: &Arc<Self>, id: u64, delay: Duration) {
        let playlist = self
            .lock()
            .jobs
            .iter()
            .find(|j| j.id == id)
            .and_then(|j| j.playlist.clone());
        match playlist {
            Some(p) => self.reap_playlist(&p.id),
            None => self.remove_jobs_after(vec![id], delay),
        }
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
        let (added_by, list, import_pos) = {
            let st = self.lock();
            st.jobs
                .iter()
                .find(|j| j.id == id)
                .map(|j| {
                    let list = j.playlist.as_ref().map(|p| p.id.clone());
                    (j.added_by.clone(), list, j.import_pos.clone())
                })
                .unwrap_or_default()
        };
        let in_playlist = list.is_some();
        self.update_job(id, |j| {
            j.status = JobStatus::Done;
            j.progress = 1.0;
            j.title = Some(track.title.clone());
        });
        let (mut queued, mut new) = (false, false);
        self.change(|st, now| {
            if st.track(&track.id).is_none() {
                track.added_by.clone_from(&added_by);
                self.insert_track(&track);
                st.library.insert(0, track.clone());
                new = true;
            }
            queued = st.push(track.clone(), &added_by, now);
            true
        });
        if new {
            self.send(library_msg(&self.lock()));
        }
        if let (Some(list), Some((order, k))) = (list, import_pos) {
            self.import_landed(&list, &order[..k], &track.id);
        }
        // A playlist import posts one summary event instead of one per song.
        if !in_playlist {
            let what = if queued {
                ""
            } else {
                " to the library (queue is full)"
            };
            self.event(&added_by, &format!("added {}{what}", track.title));
        }
        // Long enough for the client's checkmark to show.
        self.retire_job(id, Duration::from_secs(3));
    }

    pub fn fail_job(self: &Arc<Self>, id: u64, error: String) {
        self.update_job(id, |j| {
            j.status = JobStatus::Failed;
            j.error = Some(error);
        });
        self.broadcast_state(&self.lock());
        self.retire_job(id, Duration::from_secs(300));
    }

    fn remove_jobs_after(self: &Arc<Self>, ids: Vec<u64>, delay: Duration) {
        let me = Arc::clone(self);
        tokio::spawn(async move {
            tokio::time::sleep(delay).await;
            let mut st = me.lock();
            st.jobs.retain(|j| !ids.contains(&j.id));
            me.broadcast_state(&st);
        });
    }

    pub(crate) fn db(&self) -> MutexGuard<'_, Connection> {
        self.db.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn insert_track(&self, t: &Track) {
        let r = self.db().execute(
            "INSERT OR REPLACE INTO tracks
                 (id, title, duration_ms, thumbnail, added_by, added_at, gain_db)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                t.id,
                t.title,
                t.duration_ms,
                t.thumbnail,
                t.added_by,
                t.added_at,
                t.gain_db
            ],
        );
        if let Err(e) = r {
            tracing::error!("saving track: {e}");
        }
    }

    fn record_played(&self, st: &mut Station, track: Track, now: f64) {
        let db = self.db();
        let r = db
            .execute(
                "INSERT INTO history (track_id, played_at) VALUES (?1, ?2)",
                params![track.id, now as u64],
            )
            .and_then(|_| {
                db.execute(
                    "DELETE FROM history WHERE rowid NOT IN
                         (SELECT rowid FROM history ORDER BY rowid DESC LIMIT ?1)",
                    [HISTORY_LEN],
                )
            });
        if let Err(e) = r {
            tracing::error!("saving history: {e}");
        }
        st.history.insert(0, track);
        st.history.truncate(HISTORY_LEN);
    }

    /// Persist a chat message (keeping the last 500) and broadcast it.
    pub fn post_chat(&self, kind: &'static str, name: &str, text: &str) {
        let at = now_ms() as u64;
        let db = self.db(); // held while sending so `subscribe_with_history` can't interleave
        let r = db
            .execute(
                "INSERT INTO chat (kind, name, text, at) VALUES (?1, ?2, ?3, ?4)",
                params![kind, name, text, at],
            )
            .and_then(|_| {
                let id = db.last_insert_rowid();
                db.execute("DELETE FROM chat WHERE id <= ?1", [id - CHAT_KEEP])?;
                Ok(id as u64)
            });
        match r {
            Ok(id) => {
                let message = ChatMessage {
                    id,
                    kind,
                    name: name.to_string(),
                    text: text.to_string(),
                    at,
                };
                self.send(
                    json!({ "type": "chat", "message": message })
                        .to_string()
                        .into(),
                );
            }
            Err(e) => tracing::error!("saving chat: {e}"),
        }
    }

    pub fn event(&self, name: &str, text: &str) {
        self.post_chat("event", name, text);
    }

    /// Subscribe to broadcasts and fetch the last 100 chat messages (oldest first) atomically,
    /// so no message shows up both in the history and in the subscription.
    pub fn subscribe_with_history(&self) -> (broadcast::Receiver<Utf8Bytes>, Vec<ChatMessage>) {
        let db = self.db();
        let rx = self.tx.subscribe();
        let msgs = db
            .prepare(
                "SELECT id, kind, name, text, at FROM
                     (SELECT * FROM chat ORDER BY id DESC LIMIT ?1) ORDER BY id",
            )
            .and_then(|mut q| {
                q.query_map([CHAT_HISTORY], |r| {
                    let kind: String = r.get(1)?;
                    Ok(ChatMessage {
                        id: r.get(0)?,
                        kind: if kind == "user" { "user" } else { "event" },
                        name: r.get(2)?,
                        text: r.get(3)?,
                        at: r.get(4)?,
                    })
                })?
                .collect()
            })
            .unwrap_or_else(|e| {
                tracing::error!("loading chat: {e}");
                Vec::new()
            });
        (rx, msgs)
    }

    /// Ids of tracks whose loudness has not been measured yet.
    pub fn unmeasured(&self) -> Vec<String> {
        let db = self.db();
        db.prepare("SELECT id FROM tracks WHERE gain_db IS NULL")
            .and_then(|mut q| q.query_map([], |r| r.get(0))?.collect())
            .unwrap_or_default()
    }

    pub fn set_gain(&self, id: &str, gain_db: f32) {
        let r = self.db().execute(
            "UPDATE tracks SET gain_db = ?1 WHERE id = ?2",
            params![gain_db, id],
        );
        if let Err(e) = r {
            tracing::error!("saving gain: {e}");
        }
        let mut st = self.lock();
        st.set_gain(id, gain_db);
        self.broadcast_state(&st);
        self.send(library_msg(&st));
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

/// Seconds until the next local midnight (local = `TZ` env, via SQLite's `localtime`).
fn secs_to_local_midnight(db: &Connection) -> f64 {
    db.query_row(
        "SELECT unixepoch(date('now', 'localtime', '+1 day'), 'utc') - unixepoch('now', 'subsec')",
        [],
        |r| r.get(0),
    )
    .unwrap_or(3600.0)
}

/// Today's (server local day) most-reacted DJ; ties go to the most recent reaction.
fn dj_of_day(db: &Connection) -> Option<String> {
    db.query_row(
        "SELECT name FROM reactions WHERE day = date('now', 'localtime')
             ORDER BY count DESC, last_at DESC LIMIT 1",
        [],
        |r| r.get(0),
    )
    .ok()
}

fn load(db: &Connection) -> rusqlite::Result<Station> {
    db.execute_batch(
        "PRAGMA journal_mode=WAL;
         CREATE TABLE IF NOT EXISTS tracks(id TEXT PRIMARY KEY, title TEXT NOT NULL,
             duration_ms INTEGER NOT NULL, thumbnail INTEGER NOT NULL,
             added_by TEXT NOT NULL, added_at INTEGER NOT NULL);
         CREATE TABLE IF NOT EXISTS queue(queue_id INTEGER PRIMARY KEY, track_id TEXT NOT NULL,
             added_by TEXT NOT NULL, position INTEGER NOT NULL);
         CREATE TABLE IF NOT EXISTS station(k TEXT PRIMARY KEY, v TEXT NOT NULL);
         CREATE TABLE IF NOT EXISTS chat(id INTEGER PRIMARY KEY, kind TEXT NOT NULL,
             name TEXT NOT NULL, text TEXT NOT NULL, at INTEGER NOT NULL);
         CREATE TABLE IF NOT EXISTS history(track_id TEXT NOT NULL, played_at INTEGER NOT NULL);
         CREATE TABLE IF NOT EXISTS reactions(day TEXT NOT NULL, name TEXT NOT NULL,
             count INTEGER NOT NULL, last_at INTEGER NOT NULL, PRIMARY KEY(day, name));
         CREATE TABLE IF NOT EXISTS playlists(id INTEGER PRIMARY KEY AUTOINCREMENT,
             name TEXT NOT NULL, created_by TEXT NOT NULL, created_at INTEGER NOT NULL,
             updated_at INTEGER NOT NULL, source_list_id TEXT UNIQUE);
         CREATE TABLE IF NOT EXISTS playlist_tracks(playlist_id INTEGER NOT NULL,
             position INTEGER NOT NULL, track_id TEXT NOT NULL,
             PRIMARY KEY(playlist_id, position));",
    )?;
    // Phase 2 migration. NULL = not measured yet (the startup backfill looks for it).
    let has_gain: bool = db.query_row(
        "SELECT COUNT(*) > 0 FROM pragma_table_info('tracks') WHERE name = 'gain_db'",
        [],
        |r| r.get(0),
    )?;
    if !has_gain {
        db.execute_batch("ALTER TABLE tracks ADD COLUMN gain_db REAL")?;
    }
    let library = db
        .prepare("SELECT id, title, duration_ms, thumbnail, added_by, added_at, COALESCE(gain_db, 0) FROM tracks ORDER BY added_at DESC")?
        .query_map([], |r| {
            Ok(Track {
                id: r.get(0)?,
                title: r.get(1)?,
                duration_ms: r.get(2)?,
                thumbnail: r.get(3)?,
                added_by: r.get(4)?,
                added_at: r.get(5)?,
                gain_db: r.get(6)?,
            })
        })?
        .collect::<rusqlite::Result<_>>()?;
    let mut st = Station {
        library,
        dj_of_day: dj_of_day(db),
        playlists: crate::playlists::load(db)?,
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
    let played: Vec<String> = db
        .prepare("SELECT track_id FROM history ORDER BY rowid DESC LIMIT ?1")?
        .query_map([HISTORY_LEN], |r| r.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    st.history = played
        .iter()
        .filter_map(|id| st.track(id).cloned())
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
            gain_db: 0.0,
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
    fn insert_at_index() {
        let mut st = station(&["a", "b"]);
        st.current = Some(st.queue.remove(0)); // not idle
        st.queue.insert(0, st.current.clone().unwrap());
        st.insert(track("x", 1000), "u", 1, 0.0);
        assert_eq!(order(&st), ["a", "x", "b"]);
        st.insert(track("y", 1000), "u", 99, 0.0);
        assert_eq!(order(&st), ["a", "x", "b", "y"]);
        st.insert(track("z", 1000), "u", 0, 0.0);
        assert_eq!(order(&st), ["z", "a", "x", "b", "y"]);
        let mut idle = Station::default();
        idle.insert(track("a", 1000), "u", 5, 0.0);
        assert!(idle.playing && idle.current.is_some());
    }

    fn temp_cfg(tag: &str) -> (Config, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!("cb-test-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let cfg = Config {
            db_path: dir.join("t.db"),
            ..Config::from_env()
        };
        (cfg, dir)
    }

    #[test]
    fn midnight_is_within_a_day() {
        let s = secs_to_local_midnight(&Connection::open_in_memory().unwrap());
        assert!(s > 0.0 && s <= 25.0 * 3600.0, "{s}");
    }

    #[test]
    fn chat_keeps_last_500_and_serves_last_100() {
        let (cfg, dir) = temp_cfg("chat");
        let (shared, _rx) = Shared::new(cfg).unwrap();
        for i in 0..510 {
            shared.post_chat("user", "toby", &format!("m{i}"));
        }
        let n: i64 = shared
            .db()
            .query_row("SELECT COUNT(*) FROM chat", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 500);
        let (_rx, hist) = shared.subscribe_with_history();
        assert_eq!(hist.len(), 100);
        assert_eq!(hist[0].text, "m410"); // oldest first
        assert_eq!(hist[99].text, "m509");
        assert_eq!(hist[99].id, 510);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn migrates_mvp_schema() {
        let (cfg, dir) = temp_cfg("migrate");
        {
            let db = Connection::open(&cfg.db_path).unwrap();
            db.execute_batch(
                "CREATE TABLE tracks(id TEXT PRIMARY KEY, title TEXT NOT NULL,
                     duration_ms INTEGER NOT NULL, thumbnail INTEGER NOT NULL,
                     added_by TEXT NOT NULL, added_at INTEGER NOT NULL);
                 CREATE TABLE queue(queue_id INTEGER PRIMARY KEY, track_id TEXT NOT NULL,
                     added_by TEXT NOT NULL, position INTEGER NOT NULL);
                 CREATE TABLE station(k TEXT PRIMARY KEY, v TEXT NOT NULL);
                 INSERT INTO tracks VALUES ('a', 'A', 1000, 0, 'toby', 1),
                                           ('b', 'B', 1000, 0, 'toby', 2);
                 INSERT INTO queue VALUES (7, 'a', 'toby', 0);",
            )
            .unwrap();
        }
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            let (shared, _rx) = Shared::new(cfg.clone()).unwrap();
            assert_eq!(order(&shared.lock()), ["a"]);
            assert_eq!(shared.lock().library.len(), 2);
            assert_eq!(shared.unmeasured(), ["a", "b"]);
            shared.set_gain("a", 3.5);
            assert_eq!(shared.lock().queue[0].track.gain_db, 3.5);
            shared.change(|st, now| {
                st.play(now); // a becomes current
                true
            });
            shared.change(|st, now| {
                st.advance(now, true); // a leaves -> history
                true
            });
            assert_eq!(shared.lock().history[0].id, "a");
            drop(shared);
            // Reopening an already migrated DB works and keeps gain + history.
            let (shared, _rx) = Shared::new(cfg).unwrap();
            let st = shared.lock();
            assert_eq!(st.track("a").unwrap().gain_db, 3.5);
            assert_eq!(st.history.len(), 1);
            assert_eq!(st.history[0].gain_db, 3.5);
            drop(st);
            assert_eq!(shared.unmeasured(), ["b"]);
        });
        let _ = std::fs::remove_dir_all(dir);
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
                shared.enqueue(id, "toby", usize::MAX).unwrap();
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

    fn rt() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
    }

    fn job() -> Job {
        Job {
            id: 0,
            url: String::new(),
            status: JobStatus::Queued,
            progress: 0.0,
            title: None,
            error: None,
            added_by: "toby".into(),
            playlist: None,
            import_pos: None,
        }
    }

    fn playlist(ids: &[&str]) -> crate::ytdl::Playlist {
        crate::ytdl::Playlist {
            title: "Mix".into(),
            entries: ids.iter().map(|id| (id.to_string(), None)).collect(),
            skipped: 1,
            truncated: false,
        }
    }

    fn pending_in(shared: &Shared, list: &str) -> usize {
        let st = shared.lock();
        st.jobs
            .iter()
            .filter(|j| j.pending() && j.in_playlist(list))
            .count()
    }

    #[test]
    fn playlist_import_enqueues_cached_and_queues_the_rest() {
        let (cfg, dir) = temp_cfg("plimport");
        rt().block_on(async {
            let (shared, _rx) = Shared::new(cfg).unwrap();
            shared.lock().library.push(track("b", 60_000));
            let (songs, skipped, reqs) = shared
                .import_playlist("PL1", &playlist(&["a", "b", "c"]), "toby")
                .unwrap();
            assert_eq!((songs, skipped), (3, 1)); // pl.skipped carries through
            let got: Vec<&str> = reqs.iter().map(|r| r.video_id.as_str()).collect();
            assert_eq!(got, ["a", "c"], "cached b gets no job");
            let st = shared.lock();
            assert_eq!(st.current.as_ref().unwrap().track.id, "b"); // idle station starts it
            assert_eq!(st.jobs.len(), 2);
            let p = st.jobs[0].playlist.as_ref().unwrap();
            assert_eq!((p.id.as_str(), p.title.as_str()), ("PL1", "Mix"));
            assert_eq!(st.jobs[1].url, "https://www.youtube.com/watch?v=c");
        });
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn playlist_import_is_truncated_to_capacity() {
        let (cfg, dir) = temp_cfg("plcap");
        rt().block_on(async {
            let (shared, _rx) = Shared::new(cfg).unwrap();
            for _ in 0..MAX_PENDING_SINGLES {
                shared.add_job(job()).unwrap();
            }
            let ids: Vec<String> = (0..48).map(|i| format!("v{i}")).collect();
            let ids: Vec<&str> = ids.iter().map(String::as_str).collect();
            let (songs, skipped, _) = shared
                .import_playlist("PL1", &playlist(&ids), "toby")
                .unwrap();
            assert_eq!((songs, skipped), (48, 1));
            let (songs, skipped, _) = shared
                .import_playlist("PL2", &playlist(&["x", "y", "z"]), "toby")
                .unwrap();
            assert_eq!((songs, skipped), (2, 2), "only 2 of 60 slots were left");
            let err = shared
                .import_playlist("PL3", &playlist(&["q"]), "toby")
                .unwrap_err();
            assert_eq!(err, TOO_MANY_JOBS);
            assert_eq!(shared.add_job(job()).unwrap_err(), TOO_MANY_JOBS);
        });
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn playlist_jobs_do_not_count_against_single_cap() {
        let (cfg, dir) = temp_cfg("plsingle");
        rt().block_on(async {
            let (shared, _rx) = Shared::new(cfg).unwrap();
            let ids: Vec<String> = (0..40).map(|i| format!("v{i}")).collect();
            let ids: Vec<&str> = ids.iter().map(String::as_str).collect();
            shared
                .import_playlist("PL1", &playlist(&ids), "toby")
                .unwrap();
            for _ in 0..MAX_PENDING_SINGLES {
                shared.add_job(job()).unwrap();
            }
            assert_eq!(shared.add_job(job()).unwrap_err(), TOO_MANY_JOBS);
        });
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn playlist_import_into_full_queue() {
        let (cfg, dir) = temp_cfg("plfull");
        rt().block_on(async {
            let (shared, _rx) = Shared::new(cfg).unwrap();
            shared.lock().library.push(track("a", 60_000));
            for _ in 0..=MAX_QUEUE {
                shared.enqueue("a", "toby", usize::MAX).unwrap();
            }
            let err = shared
                .import_playlist("PL1", &playlist(&["a"]), "toby")
                .unwrap_err();
            assert_eq!(err, QUEUE_FULL);
        });
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn cancel_playlist_drops_only_its_queued_jobs() {
        let (cfg, dir) = temp_cfg("plcancel");
        rt().block_on(async {
            let (shared, _rx) = Shared::new(cfg).unwrap();
            let (_, _, a) = shared
                .import_playlist("PL1", &playlist(&["a", "b", "c"]), "toby")
                .unwrap();
            shared
                .import_playlist("PL2", &playlist(&["d"]), "toby")
                .unwrap();
            let single = shared.add_job(job()).unwrap();
            shared.update_job(a[0].job_id, |j| j.status = JobStatus::Downloading);
            assert_eq!(shared.cancel_playlist("PL1").as_deref(), Some("Mix"));
            assert_eq!(pending_in(&shared, "PL1"), 1, "the running download stays");
            assert_eq!(pending_in(&shared, "PL2"), 1);
            assert!(shared.lock().jobs.iter().any(|j| j.id == single));
            assert_eq!(shared.cancel_playlist("PL1"), None, "nothing queued left");
            assert_eq!(shared.cancel_playlist("nope"), None);
        });
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn finished_playlist_rows_leave_together() {
        let (cfg, dir) = temp_cfg("plreap");
        rt().block_on(async {
            tokio::time::pause();
            let (shared, _rx) = Shared::new(cfg).unwrap();
            let (_, _, r) = shared
                .import_playlist("PL1", &playlist(&["a", "b"]), "toby")
                .unwrap();
            shared.finish_job(r[0].job_id, track("a", 60_000));
            tokio::time::sleep(Duration::from_secs(30)).await;
            assert_eq!(shared.lock().jobs.len(), 2, "done row waits for the rest");
            shared.fail_job(r[1].job_id, "x".into());
            tokio::time::sleep(PLAYLIST_LINGER + Duration::from_secs(1)).await;
            assert!(shared.lock().jobs.is_empty());
        });
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn queue_is_capped() {
        let mut st = Station::default();
        for _ in 0..=MAX_QUEUE {
            assert!(st.push(track("a", 1000), "x", 0.0)); // first one becomes current
        }
        assert_eq!(st.queue.len(), MAX_QUEUE);
        assert!(!st.insert(track("b", 1000), "x", 0, 0.0));
        assert_eq!(st.queue.len(), MAX_QUEUE);

        let (cfg, dir) = temp_cfg("qcap");
        rt().block_on(async {
            let (shared, _rx) = Shared::new(cfg).unwrap();
            shared.lock().library.push(track("a", 60_000));
            for _ in 0..=MAX_QUEUE {
                shared.enqueue("a", "toby", usize::MAX).unwrap();
            }
            assert_eq!(shared.enqueue("a", "toby", 0), Err(QUEUE_FULL.into()));
            assert_eq!(shared.lock().queue.len(), MAX_QUEUE);
        });
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn pending_jobs_are_capped() {
        let (cfg, dir) = temp_cfg("jobcap");
        rt().block_on(async {
            let (shared, _rx) = Shared::new(cfg).unwrap();
            let ids: Vec<u64> = (0..MAX_PENDING_SINGLES)
                .map(|_| shared.add_job(job()).unwrap())
                .collect();
            let err = shared.add_job(job()).unwrap_err();
            assert_eq!(err, "Too many downloads queued, try again soon");
            shared.fail_job(ids[0], "x".into()); // failed jobs no longer count
            assert!(shared.add_job(job()).is_ok());
        });
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn done_job_lingers_then_goes() {
        let (cfg, dir) = temp_cfg("done");
        rt().block_on(async {
            let (shared, _rx) = Shared::new(cfg).unwrap();
            let id = shared.add_job(job()).unwrap();
            shared.finish_job(id, track("a", 60_000));
            let status = |s: &Shared| s.lock().jobs.iter().find(|j| j.id == id).map(|j| j.status);
            assert_eq!(status(&shared), Some(JobStatus::Done));
            tokio::time::sleep(Duration::from_millis(2500)).await;
            assert_eq!(status(&shared), Some(JobStatus::Done));
            tokio::time::sleep(Duration::from_millis(1000)).await;
            assert_eq!(status(&shared), None);
        });
        let _ = std::fs::remove_dir_all(dir);
    }

    /// Seeking reschedules the end timer; the stale one must not skip the next track too.
    #[test]
    fn seek_cancels_stale_end_timer() {
        let (cfg, dir) = temp_cfg("timer");
        rt().block_on(async {
            let (shared, _rx) = Shared::new(cfg).unwrap();
            shared.lock().library.push(track("a", 200)); // first timer: ~1.7 s (lead + 200 ms)
            shared.lock().library.push(track("b", 60_000));
            shared.enqueue("a", "toby", usize::MAX).unwrap(); // idle: starts playing
            shared.enqueue("b", "toby", usize::MAX).unwrap();
            tokio::time::sleep(Duration::from_millis(100)).await;
            shared.change(|st, now| {
                st.seek(now, 0.0); // new end ~0.5 s from now
                true
            });
            tokio::time::sleep(Duration::from_millis(2200)).await;
            let st = shared.lock();
            assert_eq!(st.current.as_ref().unwrap().track.id, "b");
            let played: Vec<&str> = st.history.iter().map(|t| t.id.as_str()).collect();
            assert_eq!(played, ["a"], "exactly one advance");
        });
        let _ = std::fs::remove_dir_all(dir);
    }
}
