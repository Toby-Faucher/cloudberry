//! YouTube URL validation and the single download worker (yt-dlp).

use crate::station::{Job, JobStatus, QUEUE_FULL, Shared, Track, now_ms};
use axum::extract::ws::Utf8Bytes;
use std::path::Path;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::mpsc;

const HOSTS: [&str; 5] = [
    "youtube.com",
    "www.youtube.com",
    "m.youtube.com",
    "music.youtube.com",
    "youtu.be",
];
const MAX_DURATION_MS: u64 = 20 * 60 * 1000;

pub fn is_video_id(s: &str) -> bool {
    s.len() == 11
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

/// Strictly split a YouTube URL into (host, path segments, query string).
fn parse_yt(input: &str) -> Option<(&'static str, Vec<&str>, &str)> {
    let s = input.trim();
    if s.len() > 2048 || s.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return None;
    }
    let rest = match s.find("://") {
        Some(i) => {
            let scheme = &s[..i];
            if !scheme.eq_ignore_ascii_case("https") && !scheme.eq_ignore_ascii_case("http") {
                return None;
            }
            &s[i + 3..]
        }
        None => s, // bare "youtube.com/watch?v=..."
    };
    let rest = rest.split('#').next()?;
    let auth_end = rest.find(['/', '?']).unwrap_or(rest.len());
    let (authority, path_query) = rest.split_at(auth_end);
    if authority.contains(['@', '\\', '%']) {
        return None;
    }
    let host = match authority.rsplit_once(':') {
        Some((h, "443" | "80")) => h,
        Some(_) => return None,
        None => authority,
    }
    .to_ascii_lowercase();
    let host = host.strip_suffix('.').unwrap_or(&host);
    let host = *HOSTS.iter().find(|h| **h == host)?;
    let (path, query) = path_query.split_once('?').unwrap_or((path_query, ""));
    let segs = path.split('/').filter(|p| !p.is_empty()).collect();
    Some((host, segs, query))
}

fn query_param<'a>(query: &'a str, key: &str) -> Option<&'a str> {
    query
        .split('&')
        .find_map(|kv| kv.strip_prefix(key)?.strip_prefix('='))
}

/// Strictly parse a YouTube URL and return its 11-char video id.
pub fn video_id(input: &str) -> Option<String> {
    let (host, segs, query) = parse_yt(input)?;
    let id = if host == "youtu.be" {
        match segs.as_slice() {
            [id] => *id,
            _ => return None,
        }
    } else {
        match segs.as_slice() {
            ["watch"] => query_param(query, "v").unwrap_or(""),
            ["shorts" | "embed" | "live" | "v", id] => id,
            _ => return None,
        }
    };
    is_video_id(id).then(|| id.to_string())
}

fn is_list_id(s: &str) -> bool {
    (2..=64).contains(&s.len())
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

/// `youtube.com/playlist?list=<id>` (any YouTube host but youtu.be) -> the list id.
/// `watch?v=..&list=..` stays a single-video add.
pub fn playlist_id(input: &str) -> Option<String> {
    let (host, segs, query) = parse_yt(input)?;
    if host == "youtu.be" || segs != ["playlist"] {
        return None;
    }
    query_param(query, "list")
        .filter(|id| is_list_id(id))
        .map(str::to_string)
}

pub fn playlist_url(id: &str) -> String {
    format!("https://www.youtube.com/playlist?list={id}")
}

pub fn canonical_url(id: &str) -> String {
    format!("https://www.youtube.com/watch?v={id}")
}

#[derive(Debug)]
pub struct DlRequest {
    pub job_id: u64,
    pub video_id: String,
}

/// Single download worker: processes one request at a time.
pub async fn worker(shared: Arc<Shared>, mut rx: mpsc::UnboundedReceiver<DlRequest>) {
    while let Some(req) = rx.recv().await {
        if !shared.lock().jobs.iter().any(|j| j.id == req.job_id) {
            continue; // cancelled (cancel_playlist) before it started
        }
        // A duplicate add may have been queued while the first one downloaded.
        let existing = shared.lock().track(&req.video_id).cloned();
        let result = match existing {
            Some(t) => Ok(t),
            None => {
                shared.update_job(req.job_id, |j| j.status = JobStatus::Downloading);
                match download(&shared, req.job_id, &req.video_id).await {
                    Ok(mut t) => {
                        let mp3 = shared.cfg.music_dir.join(format!("{}.mp3", t.id));
                        match crate::loudness::measure(&mp3).await {
                            Ok(g) => t.gain_db = g,
                            // shortcut: stored as 0 and never retried; fine while ffmpeg works for yt-dlp too
                            Err(e) => tracing::warn!(id = %t.id, "loudness: {e}"),
                        }
                        Ok(t)
                    }
                    Err(e) => {
                        remove_partials(&shared.cfg.music_dir, &req.video_id);
                        Err(e)
                    }
                }
            }
        };
        match result {
            Ok(track) => shared.finish_job(req.job_id, track),
            Err(e) => {
                tracing::warn!(id = %req.video_id, "download failed: {e}");
                shared.fail_job(req.job_id, e);
            }
        }
    }
}

async fn download(shared: &Arc<Shared>, job_id: u64, id: &str) -> Result<Track, String> {
    let dir = &shared.cfg.music_dir;
    let out = format!(
        "{}/%(id)s.%(ext)s",
        dir.display().to_string().replace('%', "%%")
    );
    let mut child = Command::new(&shared.cfg.ytdlp)
        .args([
            "--no-playlist",
            "--no-colors",
            "--match-filter",
            "duration<=1200",
            "-x",
            "--audio-format",
            "mp3",
            "--audio-quality",
            "2",
            "--write-thumbnail",
            "--convert-thumbnails",
            "jpg",
            "--newline",
            "--progress",
            "--progress-template",
            "download:cbp %(progress.downloaded_bytes)s %(progress.total_bytes)s %(progress.total_bytes_estimate)s",
            "-o",
            &out,
            "--print-json",
            "--",
        ])
        .arg(canonical_url(id)) // rebuilt from a validated id, never user input
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        // Own process group: yt-dlp_linux is a PyInstaller bootloader with a Python child that
        // spawns ffmpeg, so a timeout must kill the whole group, not just the direct child.
        .process_group(0)
        .spawn()
        .map_err(|e| format!("could not run yt-dlp: {e}"))?;

    let (tx, mut lines) = mpsc::unbounded_channel::<String>();
    for pipe in [
        child
            .stdout
            .take()
            .map(|p| Box::new(p) as Box<dyn tokio::io::AsyncRead + Unpin + Send>),
        child.stderr.take().map(|p| Box::new(p) as _),
    ]
    .into_iter()
    .flatten()
    {
        let tx = tx.clone();
        tokio::spawn(async move {
            let mut r = BufReader::new(pipe).lines();
            while let Ok(Some(l)) = r.next_line().await {
                let _ = tx.send(l);
            }
        });
    }
    drop(tx);

    let mut info: Option<serde_json::Value> = None;
    let mut last_err = String::new();
    let mut sent = 0.0f32;
    let run = async {
        while let Some(line) = lines.recv().await {
            if let Some(p) = line.strip_prefix("cbp ") {
                if let Some(f) = parse_progress(p)
                    && (f - sent >= 0.02 || f >= 0.99 && sent < 0.99)
                {
                    sent = f;
                    shared.update_job(job_id, |j| j.progress = f.min(0.99));
                }
            } else if line.starts_with('{') {
                info = serde_json::from_str(&line).ok();
            } else if let Some(e) = line.strip_prefix("ERROR: ") {
                last_err = e.chars().take(300).collect();
            }
        }
        child.wait().await
    };
    let status = match tokio::time::timeout(shared.cfg.download_timeout, run).await {
        Ok(s) => s.map_err(|e| e.to_string())?,
        Err(_) => {
            kill_tree(&mut child).await;
            return Err("download timed out".into());
        }
    };
    if !status.success() {
        return Err(if last_err.is_empty() {
            format!("yt-dlp exited with {status}")
        } else {
            last_err
        });
    }
    let mp3 = dir.join(format!("{id}.mp3"));
    if !mp3.is_file() {
        // --match-filter skips silently (no JSON printed) when duration > 20 min or unknown.
        return Err(if info.is_none() {
            "video is longer than 20 minutes (or has no duration)"
        } else {
            "yt-dlp produced no mp3"
        }
        .into());
    }
    let info = info.unwrap_or_default();
    let duration_ms = (info["duration"].as_f64().unwrap_or(0.0) * 1000.0).round() as u64;
    if duration_ms == 0 || duration_ms > MAX_DURATION_MS {
        let _ = std::fs::remove_file(&mp3);
        return Err("video duration missing or over 20 minutes".into());
    }
    let title = info["title"]
        .as_str()
        .unwrap_or(id)
        .chars()
        .take(200)
        .collect();
    Ok(Track {
        id: id.to_string(),
        title,
        duration_ms,
        thumbnail: dir.join(format!("{id}.jpg")).is_file(),
        added_by: String::new(), // filled in by finish_job
        added_at: now_ms() as u64,
        gain_db: 0.0, // measured by the worker
    })
}

/// Delete `{id}.*` leftovers (.part, .webm, .jpg, half-converted .mp3) after a failed download.
fn remove_partials(dir: &Path, id: &str) {
    let prefix = format!("{id}.");
    for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        if e.file_name().to_string_lossy().starts_with(&prefix) {
            let _ = std::fs::remove_file(e.path());
        }
    }
}

/// "<downloaded> <total> <estimate>" (any may be "NA") -> fraction 0..1.
fn parse_progress(p: &str) -> Option<f32> {
    let mut it = p.split_whitespace().map(|v| v.parse::<f64>().ok());
    let done = it.next()??;
    let total = it.next().flatten().or(it.next().flatten())?;
    (total > 0.0).then(|| (done / total).clamp(0.0, 1.0) as f32)
}

/// Kill a child spawned with `process_group(0)` together with its whole group, then reap it.
/// yt-dlp_linux is a PyInstaller bootloader whose Python child spawns ffmpeg, so killing just
/// the direct child would leave the rest running.
async fn kill_tree(child: &mut Child) {
    if let Some(pid) = child.id() {
        // SAFETY: plain syscall; pgid == pid thanks to process_group(0).
        unsafe { libc::killpg(pid as libc::pid_t, libc::SIGKILL) };
    }
    let _ = child.wait().await; // reap; grandchildren go to init (compose `init: true`)
}

pub const PLAYLIST_MAX: usize = 50;

/// A `--flat-playlist -J` result, filtered to what we will try to add.
#[derive(Debug, PartialEq)]
pub struct Playlist {
    pub title: String,
    /// (video id, title if known), in playlist order.
    pub entries: Vec<(String, Option<String>)>,
    /// Entries without a usable id, or known to be longer than 20 minutes.
    pub skipped: usize,
    /// The playlist has more than the entries yt-dlp returned (`--playlist-end`).
    pub truncated: bool,
}

pub fn parse_playlist(json: &str) -> Result<Playlist, String> {
    let v: serde_json::Value =
        serde_json::from_str(json).map_err(|_| "Couldn't read that playlist".to_string())?;
    let entries = v["entries"]
        .as_array()
        .ok_or("That link isn't a playlist")?;
    let mut out = Playlist {
        title: v["title"]
            .as_str()
            .unwrap_or("playlist")
            .chars()
            .take(200)
            .collect(),
        entries: Vec::new(),
        skipped: 0,
        truncated: entries.len() > PLAYLIST_MAX
            || v["playlist_count"]
                .as_u64()
                .is_some_and(|n| n as usize > entries.len()),
    };
    for e in entries.iter().take(PLAYLIST_MAX) {
        let too_long = e["duration"]
            .as_f64()
            .is_some_and(|d| d * 1000.0 > MAX_DURATION_MS as f64);
        match e["id"].as_str().filter(|id| is_video_id(id)) {
            Some(id) if !too_long => {
                let title = e["title"].as_str().map(|t| t.chars().take(200).collect());
                out.entries.push((id.to_string(), title));
            }
            _ => out.skipped += 1,
        }
    }
    Ok(out)
}

/// Run `yt-dlp --flat-playlist -J` (one subprocess, killed with its group on timeout).
async fn expand(shared: &Shared, list_id: &str) -> Result<Playlist, String> {
    let mut child = Command::new(&shared.cfg.ytdlp)
        .args([
            "--flat-playlist",
            "-J",
            "--playlist-end",
            "50",
            "--no-colors",
            "--",
        ])
        .arg(playlist_url(list_id)) // rebuilt from a validated id, never user input
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .process_group(0)
        .spawn()
        .map_err(|e| format!("could not run yt-dlp: {e}"))?;
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let (stdout, stderr) = (child.stdout.take(), child.stderr.take());
    let run = async {
        if let (Some(o), Some(e)) = (stdout, stderr) {
            // Caps: a 50-entry flat playlist is ~100 KB; past that something is wrong.
            let (mut o, mut e) = (o.take(16 << 20), e.take(64 << 10));
            let _ = tokio::join!(o.read_to_end(&mut out), e.read_to_end(&mut err));
        }
        child.wait().await
    };
    let status = match tokio::time::timeout(shared.cfg.playlist_timeout, run).await {
        Ok(s) => s.map_err(|e| e.to_string())?,
        Err(_) => {
            kill_tree(&mut child).await;
            return Err("Loading the playlist timed out".into());
        }
    };
    if !status.success() {
        let err = String::from_utf8_lossy(&err);
        let why = err
            .lines()
            .rev()
            .find_map(|l| l.strip_prefix("ERROR: "))
            .unwrap_or("yt-dlp failed");
        return Err(format!(
            "Couldn't load that playlist: {}",
            why.chars().take(300).collect::<String>()
        ));
    }
    parse_playlist(&String::from_utf8_lossy(&out))
}

/// Expand a playlist, enqueue what's cached, queue downloads for the rest, post the event.
async fn import_playlist(
    shared: &Arc<Shared>,
    list_id: &str,
    added_by: &str,
) -> Result<(), String> {
    // One expansion at a time server-wide: each is a yt-dlp process.
    let _permit = shared
        .expanding
        .try_acquire()
        .map_err(|_| "Another playlist is loading, try again soon")?;
    let pl = expand(shared, list_id).await?;
    let (songs, skipped, requests) = shared.import_playlist(list_id, &pl, added_by)?;
    for r in requests {
        shared
            .dl_tx
            .send(r)
            .map_err(|_| "download worker is not running".to_string())?;
    }
    let first = if pl.truncated {
        format!(" (first {})", pl.entries.len() + pl.skipped)
    } else {
        String::new()
    };
    shared.event(
        added_by,
        &format!(
            "added playlist {} ({songs} song{}, {skipped} skipped){first}",
            pl.title,
            if songs == 1 { "" } else { "s" }
        ),
    );
    Ok(())
}

/// Handle an `add`: a playlist link expands in the background (errors go to `reply`); a video
/// link creates a queued job, or enqueues straight away if the track is already in the library.
pub fn handle_add(
    shared: &Arc<Shared>,
    url: &str,
    added_by: &str,
    reply: &mpsc::UnboundedSender<Utf8Bytes>,
) -> Result<(), String> {
    if let Some(list) = playlist_id(url) {
        let (shared, added_by, reply) = (Arc::clone(shared), added_by.to_string(), reply.clone());
        tokio::spawn(async move {
            if let Err(e) = import_playlist(&shared, &list, &added_by).await {
                let _ = reply.send(crate::error_msg(&e));
            }
        });
        return Ok(());
    }
    let id = video_id(url).ok_or("Not a valid YouTube video link")?;
    match shared.enqueue(&id, added_by, usize::MAX) {
        Ok(()) => return Ok(()),
        Err(e) if e == QUEUE_FULL => return Err(e),
        Err(_) => {} // not in the library yet: download it
    }
    let job = Job {
        id: 0,
        url: canonical_url(&id),
        status: JobStatus::Queued,
        progress: 0.0,
        title: None,
        error: None,
        added_by: added_by.to_string(),
        playlist: None,
    };
    let job_id = shared.add_job(job)?;
    shared
        .dl_tx
        .send(DlRequest {
            job_id,
            video_id: id,
        })
        .map_err(|_| "download worker is not running".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const ID: &str = "jNQXAC9IVzw";

    #[test]
    fn accepts_youtube_forms() {
        for u in [
            "https://www.youtube.com/watch?v=jNQXAC9IVzw",
            "http://youtube.com/watch?v=jNQXAC9IVzw",
            "https://m.youtube.com/watch?v=jNQXAC9IVzw&t=4s",
            "https://music.youtube.com/watch?v=jNQXAC9IVzw&list=RDAMVM",
            "https://www.youtube.com/watch?list=PL123&v=jNQXAC9IVzw&index=2",
            "https://youtu.be/jNQXAC9IVzw",
            "https://youtu.be/jNQXAC9IVzw?si=abc&t=10",
            "https://www.youtube.com/shorts/jNQXAC9IVzw",
            "https://www.youtube.com/embed/jNQXAC9IVzw",
            "https://www.youtube.com/live/jNQXAC9IVzw?feature=share",
            "HTTPS://WWW.YOUTUBE.COM/watch?v=jNQXAC9IVzw",
            "www.youtube.com/watch?v=jNQXAC9IVzw",
            "youtu.be/jNQXAC9IVzw",
            "  https://www.youtube.com/watch?v=jNQXAC9IVzw#frag  ",
            "https://www.youtube.com:443/watch?v=jNQXAC9IVzw",
            "https://www.youtube.com./watch?v=jNQXAC9IVzw",
        ] {
            assert_eq!(video_id(u).as_deref(), Some(ID), "{u}");
        }
    }

    #[test]
    fn rejects_bad_urls() {
        for u in [
            "",
            "jNQXAC9IVzw",
            "https://youtube.com.evil.com/watch?v=jNQXAC9IVzw",
            "https://evil.com/youtube.com/watch?v=jNQXAC9IVzw",
            "https://evilyoutube.com/watch?v=jNQXAC9IVzw",
            "https://www.youtube.com@evil.com/watch?v=jNQXAC9IVzw",
            "https://user@www.youtube.com/watch?v=jNQXAC9IVzw",
            "https://evil.com\\@youtube.com/watch?v=jNQXAC9IVzw",
            "https://www.youtube.com:8080/watch?v=jNQXAC9IVzw",
            "ftp://www.youtube.com/watch?v=jNQXAC9IVzw",
            "javascript://youtu.be/jNQXAC9IVzw",
            "https://www.youtube.com/playlist?list=PL123",
            "https://www.youtube.com/watch?v=jNQXAC9IVz",
            "https://www.youtube.com/watch?v=jNQXAC9IVzwX",
            "https://www.youtube.com/watch?v=--exec=rm",
            "https://www.youtube.com/watch?v=jNQXAC9IV$w",
            "https://www.youtube.com/watch?vv=jNQXAC9IVzw",
            "https://www.youtube.com/channel/jNQXAC9IVzw",
            "https://youtu.be/",
            "https://youtu.be/jNQXAC9IVzw/extra",
            "https://www.youtube.com/watch?v=jNQXAC9IVzw extra",
            "https://www.you%74ube.com/watch?v=jNQXAC9IVzw",
            "--exec rm -rf /",
        ] {
            assert_eq!(video_id(u), None, "{u}");
        }
    }

    #[test]
    fn canonical() {
        assert_eq!(
            canonical_url(ID),
            "https://www.youtube.com/watch?v=jNQXAC9IVzw"
        );
    }

    #[test]
    fn playlist_urls() {
        let id = "PLKM3pENfZPOLCUSim2fut61AWCp9AUrQU";
        for u in [
            format!("https://www.youtube.com/playlist?list={id}"),
            format!("https://m.youtube.com/playlist?list={id}&si=x"),
            format!("https://music.youtube.com/playlist?feature=share&list={id}"),
            format!("youtube.com/playlist?list={id}#top"),
            "https://www.youtube.com/playlist?list=RDAMVM_-x".into(),
        ] {
            assert!(playlist_id(&u).is_some(), "{u}");
        }
        assert_eq!(
            playlist_id(&format!("https://www.youtube.com/playlist?list={id}")).as_deref(),
            Some(id)
        );
        for u in [
            "https://www.youtube.com/playlist?list=P".to_string(), // too short
            format!("https://www.youtube.com/playlist?list={}", "a".repeat(65)),
            "https://www.youtube.com/playlist?list=PL1;rm".into(),
            "https://www.youtube.com/playlist?list=--exec=rm".into(),
            "https://www.youtube.com/playlist?list=PL%20x".into(),
            "https://www.youtube.com/playlist?list=PL12 3".into(),
            "https://www.youtube.com/playlist?lists=PL123".into(),
            "https://www.youtube.com/playlist".into(),
            format!("https://youtu.be/playlist?list={id}"),
            format!("https://youtube.com.evil.com/playlist?list={id}"),
            format!("https://evil.com/www.youtube.com/playlist?list={id}"),
            format!("https://user@www.youtube.com/playlist?list={id}"),
            format!("https://www.youtube.com:8080/playlist?list={id}"),
            format!("ftp://www.youtube.com/playlist?list={id}"),
            format!("https://www.youtube.com/playlist/x?list={id}"),
            format!("https://www.youtube.com/watch?v=jNQXAC9IVzw&list={id}"),
        ] {
            assert_eq!(playlist_id(&u), None, "{u}");
        }
        // Even an id that looks like a flag only ever reaches yt-dlp inside the rebuilt URL,
        // after "--".
        assert_eq!(
            playlist_id("https://www.youtube.com/playlist?list=--exec").map(|id| playlist_url(&id)),
            Some("https://www.youtube.com/playlist?list=--exec".into())
        );
        // watch?v=..&list=.. stays a single video.
        assert_eq!(
            video_id(&format!("https://www.youtube.com/watch?v={ID}&list={id}")).as_deref(),
            Some(ID)
        );
        assert_eq!(
            playlist_url("PL_a-1"),
            "https://www.youtube.com/playlist?list=PL_a-1"
        );
    }

    const FIXTURE: &str = include_str!("../tests/fixtures/flat_playlist.json");

    #[test]
    fn parses_captured_flat_playlist() {
        let pl = parse_playlist(FIXTURE).unwrap();
        assert_eq!(pl.title, "10 sec Intro Music");
        let ids: Vec<&str> = pl.entries.iter().map(|(id, _)| id.as_str()).collect();
        assert_eq!(
            ids,
            [
                "NOaZVn2ML44",
                "9MnGSJYyR8Q",
                "TbSwCoThlbo",
                "hiu9WSl4OKg",
                "lhYmGBmO0bs" // unavailable: no title/duration, still tried (and fails as a job)
            ]
        );
        assert_eq!(pl.entries[0].1.as_deref(), Some("Good intro Music"));
        assert_eq!(pl.entries[4].1, None);
        assert_eq!(pl.skipped, 0);
        assert!(
            pl.truncated,
            "fixture was trimmed to 5 of playlist_count 25"
        );
    }

    #[test]
    fn playlist_skips_long_and_idless_entries_and_caps_at_50() {
        let mut v: serde_json::Value = serde_json::from_str(FIXTURE).unwrap();
        v["playlist_count"] = 5.into();
        v["entries"][1]["duration"] = 1201.into();
        v["entries"][2]["id"] = "not-an-id".into();
        v["entries"][3].as_object_mut().unwrap().remove("id");
        let pl = parse_playlist(&v.to_string()).unwrap();
        assert_eq!(pl.entries.len(), 2);
        assert_eq!(pl.skipped, 3);
        assert!(!pl.truncated);

        let one = v["entries"][0].clone();
        v["entries"] = vec![one; 60].into();
        v["playlist_count"] = serde_json::Value::Null;
        let pl = parse_playlist(&v.to_string()).unwrap();
        assert_eq!(pl.entries.len(), PLAYLIST_MAX);
        assert!(pl.truncated);

        assert!(parse_playlist("{}").is_err());
        assert!(parse_playlist("not json").is_err());
    }

    #[test]
    fn progress_parsing() {
        assert_eq!(parse_progress("50 100 NA"), Some(0.5));
        assert_eq!(parse_progress("25 NA 100"), Some(0.25));
        assert_eq!(parse_progress("25 NA NA"), None);
        assert_eq!(parse_progress("NA 100 NA"), None);
    }
}
