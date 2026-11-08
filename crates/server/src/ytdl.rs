//! YouTube URL validation and the single download worker (yt-dlp).

use crate::station::{Job, JobStatus, Shared, Track, now_ms};
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
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

/// Strictly parse a YouTube URL and return its 11-char video id.
pub fn video_id(input: &str) -> Option<String> {
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
    if !HOSTS.contains(&host) {
        return None;
    }
    let (path, query) = path_query.split_once('?').unwrap_or((path_query, ""));
    let segs: Vec<&str> = path.split('/').filter(|p| !p.is_empty()).collect();
    let id = if host == "youtu.be" {
        match segs.as_slice() {
            [id] => *id,
            _ => return None,
        }
    } else {
        match segs.as_slice() {
            ["watch"] => query
                .split('&')
                .find_map(|kv| kv.strip_prefix("v="))
                .unwrap_or(""),
            ["shorts" | "embed" | "live" | "v", id] => id,
            _ => return None,
        }
    };
    is_video_id(id).then(|| id.to_string())
}

pub fn canonical_url(id: &str) -> String {
    format!("https://www.youtube.com/watch?v={id}")
}

pub struct DlRequest {
    pub job_id: u64,
    pub video_id: String,
}

/// Single download worker: processes one request at a time.
pub async fn worker(shared: Arc<Shared>, mut rx: mpsc::UnboundedReceiver<DlRequest>) {
    while let Some(req) = rx.recv().await {
        // A duplicate add may have been queued while the first one downloaded.
        let existing = shared.lock().track(&req.video_id).cloned();
        let result = match existing {
            Some(t) => Ok(t),
            None => {
                shared.update_job(req.job_id, |j| j.status = JobStatus::Downloading);
                match tokio::time::timeout(
                    Duration::from_secs(900),
                    download(&shared, req.job_id, &req.video_id),
                )
                .await
                {
                    Ok(Ok(mut t)) => {
                        let mp3 = shared.cfg.music_dir.join(format!("{}.mp3", t.id));
                        match crate::loudness::measure(&mp3).await {
                            Ok(g) => t.gain_db = g,
                            // shortcut: stored as 0 and never retried; fine while ffmpeg works for yt-dlp too
                            Err(e) => tracing::warn!(id = %t.id, "loudness: {e}"),
                        }
                        Ok(t)
                    }
                    Ok(Err(e)) => Err(e),
                    Err(_) => Err("download timed out".into()),
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
    let status = child.wait().await.map_err(|e| e.to_string())?;
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

/// "<downloaded> <total> <estimate>" (any may be "NA") -> fraction 0..1.
fn parse_progress(p: &str) -> Option<f32> {
    let mut it = p.split_whitespace().map(|v| v.parse::<f64>().ok());
    let done = it.next()??;
    let total = it.next().flatten().or(it.next().flatten())?;
    (total > 0.0).then(|| (done / total).clamp(0.0, 1.0) as f32)
}

/// Create a queued job, or enqueue straight away if the track is already in the library.
pub fn handle_add(shared: &Arc<Shared>, url: &str, added_by: &str) -> Result<(), String> {
    let id = video_id(url).ok_or("Not a valid YouTube video link")?;
    if shared.enqueue(&id, added_by, usize::MAX).is_ok() {
        return Ok(());
    }
    let job = Job {
        id: 0,
        url: canonical_url(&id),
        status: JobStatus::Queued,
        progress: 0.0,
        title: None,
        error: None,
        added_by: added_by.to_string(),
    };
    let job_id = shared.add_job(job);
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
    fn progress_parsing() {
        assert_eq!(parse_progress("50 100 NA"), Some(0.5));
        assert_eq!(parse_progress("25 NA 100"), Some(0.25));
        assert_eq!(parse_progress("25 NA NA"), None);
        assert_eq!(parse_progress("NA 100 NA"), None);
    }
}
