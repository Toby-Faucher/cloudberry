//! Loudness normalisation: one ffmpeg `ebur128` pass per track -> `Track.gain_db`.

use crate::station::Shared;
use std::path::Path;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;
use tokio::process::Command;

const TARGET_LUFS: f32 = -14.0;

/// Measure integrated loudness of `path` and return the gain that brings it to -14 LUFS.
pub async fn measure(path: &Path) -> Result<f32, String> {
    let run = Command::new("ffmpeg")
        .args(["-hide_banner", "-nostdin", "-nostats", "-i"])
        .arg(path)
        .args([
            "-filter_complex",
            "ebur128=framelog=quiet",
            "-f",
            "null",
            "-",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .kill_on_drop(true)
        .output();
    let out = tokio::time::timeout(Duration::from_secs(300), run)
        .await
        .map_err(|_| "ffmpeg timed out".to_string())?
        .map_err(|e| format!("could not run ffmpeg: {e}"))?;
    let i = integrated_loudness(&String::from_utf8_lossy(&out.stderr))
        .ok_or_else(|| format!("no loudness in ffmpeg output ({})", out.status))?;
    Ok(gain_for(i))
}

/// `I: <x> LUFS` from the ebur128 summary.
fn integrated_loudness(stderr: &str) -> Option<f32> {
    stderr
        .lines()
        .skip_while(|l| !l.contains("Integrated loudness:"))
        .find_map(|l| l.trim().strip_prefix("I:"))?
        .split_whitespace()
        .next()?
        .parse()
        .ok()
}

fn gain_for(integrated: f32) -> f32 {
    (TARGET_LUFS - integrated).clamp(-12.0, 12.0)
}

/// Measure library tracks that predate loudness support, one at a time.
pub async fn backfill(shared: Arc<Shared>) {
    for id in shared.unmeasured() {
        let path = shared.cfg.music_dir.join(format!("{id}.mp3"));
        match measure(&path).await {
            Ok(g) => {
                tracing::info!(%id, gain_db = g, "measured loudness");
                shared.set_gain(&id, g);
            }
            Err(e) => tracing::warn!(%id, "loudness measurement failed: {e}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_real_ffmpeg_output() {
        // Captured from ffmpeg 9.0 on a 3 s 440 Hz sine mp3.
        let sample = include_str!("../tests/fixtures/ebur128_stderr.txt");
        assert_eq!(integrated_loudness(sample), Some(-22.2));
        assert!((gain_for(-22.2) - 8.2).abs() < 1e-4);
        assert_eq!(integrated_loudness("Threshold: -32.2 LUFS\n    I: x"), None);
        assert_eq!(integrated_loudness(""), None);
    }

    #[test]
    fn gain_is_clamped() {
        assert_eq!(gain_for(-70.0), 12.0);
        assert_eq!(gain_for(0.0), -12.0);
        assert_eq!(gain_for(-14.0), 0.0);
    }
}
