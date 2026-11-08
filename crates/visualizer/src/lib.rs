//! Audio analysis for the Cloudberry visualizer, exported as a raw WASM ABI
//! (no wasm-bindgen). JS writes FFT_SIZE samples at `input_ptr()`, calls
//! `process(sample_rate, dt)`, then reads BARS floats at `output_ptr()` / `peaks_ptr()`,
//! WAVE_POINTS floats at `waveform_ptr()`, and the scalars `energy`/`bass`/`mid`/`treble`/`beat`.

use std::sync::Arc;

use realfft::num_complex::Complex32;
use realfft::{RealFftPlanner, RealToComplex};

pub const FFT_SIZE: usize = 2048;
pub const BARS: usize = 64;
pub const WAVE_POINTS: usize = 256;
const BINS: usize = FFT_SIZE / 2 + 1;

const MIN_HZ: f32 = 30.0;
const MAX_HZ: f32 = 16_000.0;
const DB_FLOOR: f32 = -90.0;
const DB_CEIL: f32 = -10.0;
/// Energy is RMS loudness mapped from this many dBFS up to 0 dBFS.
const ENERGY_FLOOR_DB: f32 = -60.0;
/// Time constants (seconds) for rising / falling bars.
const ATTACK_TAU: f32 = 0.03;
const DECAY_TAU: f32 = 0.25;
const ENERGY_TAU: f32 = 0.15;
/// Peak markers fall at this many units (of 0..1) per second.
const PEAK_FALL_PER_S: f32 = 0.6;

/// Band ranges (Hz) for the `bass` / `mid` / `treble` summaries.
const BASS_HZ: (f32, f32) = (30.0, 150.0);
const MID_HZ: (f32, f32) = (150.0, 2_000.0);
const TREBLE_HZ: (f32, f32) = (2_000.0, 16_000.0);

/// Beat detection: spectral flux over the bars below this frequency (low + low-mid).
const FLUX_MAX_HZ: f32 = 500.0;
/// Threshold = mean + FLUX_K * stddev of the last FLUX_WINDOW_S of flux values.
const FLUX_K: f32 = 1.5;
const FLUX_WINDOW_S: f32 = 1.0;
/// Absolute minimum flux (mean positive rise per band, in 0..1 units), so near-steady
/// signals with a tiny stddev never trigger.
const FLUX_FLOOR: f32 = 0.03;
const REFRACTORY_S: f32 = 0.2;
const BEAT_DECAY_S: f32 = 0.25;
/// Flux history ring; 128 frames covers 1 s at up to ~120 fps.
const FLUX_HISTORY: usize = 128;

pub struct Analyzer {
    pub input: [f32; FFT_SIZE],
    pub bars: [f32; BARS],
    pub peaks: [f32; BARS],
    pub energy: f32,
    pub bass: f32,
    pub mid: f32,
    pub treble: f32,
    pub waveform: [f32; WAVE_POINTS],
    pub beat: BeatDetector,
    window: [f32; FFT_SIZE],
    window_gain: f32,
    fft: Arc<dyn RealToComplex<f32>>,
    buf: Vec<f32>,
    spectrum: Vec<Complex32>,
    scratch: Vec<Complex32>,
    levels: [f32; BARS],
}

impl Default for Analyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer {
    /// All allocation happens here, once; `process` allocates nothing.
    pub fn new() -> Self {
        let fft = RealFftPlanner::<f32>::new().plan_fft_forward(FFT_SIZE);
        let mut window = [0.0; FFT_SIZE];
        for (i, w) in window.iter_mut().enumerate() {
            *w = 0.5 - 0.5 * (2.0 * std::f32::consts::PI * i as f32 / FFT_SIZE as f32).cos();
        }
        let window_gain = window.iter().sum();
        Self {
            input: [0.0; FFT_SIZE],
            bars: [0.0; BARS],
            peaks: [0.0; BARS],
            energy: 0.0,
            bass: 0.0,
            mid: 0.0,
            treble: 0.0,
            waveform: [0.0; WAVE_POINTS],
            beat: BeatDetector::default(),
            window,
            window_gain,
            buf: fft.make_input_vec(),
            spectrum: fft.make_output_vec(),
            scratch: fft.make_scratch_vec(),
            fft,
            levels: [0.0; BARS],
        }
    }

    pub fn process(&mut self, sample_rate: f32, dt: f32) {
        let dt = if dt.is_finite() { dt.clamp(0.0, 1.0) } else { 0.0 };
        for ((b, s), w) in self.buf.iter_mut().zip(&self.input).zip(&self.window) {
            *b = if s.is_finite() { s * w } else { 0.0 };
        }
        // Only fails on wrong buffer lengths, which `new` rules out.
        let _ = self
            .fft
            .process_with_scratch(&mut self.buf, &mut self.spectrum, &mut self.scratch);

        // Amplitude 1.0 sine -> 1.0 after this scaling (i.e. 0 dBFS).
        let scale = 2.0 / self.window_gain;
        let mut mags = [0.0f32; BINS];
        for (m, c) in mags.iter_mut().zip(&self.spectrum) {
            *m = c.norm() * scale;
        }
        band_levels(&mags, sample_rate, &mut self.levels);

        for i in 0..BARS {
            self.bars[i] = smooth(self.bars[i], self.levels[i], dt);
            self.peaks[i] = fall_peak(self.peaks[i], self.bars[i], dt);
        }
        let target = rms_level(&self.input);
        self.energy += (target - self.energy) * (1.0 - (-dt / ENERGY_TAU).exp());

        self.bass = smooth(self.bass, range_mean(&self.levels, BASS_HZ), dt);
        self.mid = smooth(self.mid, range_mean(&self.levels, MID_HZ), dt);
        self.treble = smooth(self.treble, range_mean(&self.levels, TREBLE_HZ), dt);
        self.beat.update(&self.levels, dt);

        let step = FFT_SIZE / WAVE_POINTS;
        for (w, chunk) in self.waveform.iter_mut().zip(self.input.chunks_exact(step)) {
            let sum: f32 = chunk.iter().filter(|s| s.is_finite()).sum();
            *w = (sum / step as f32).clamp(-1.0, 1.0);
        }
    }
}

/// Mean level of the bars whose centre frequency lies in `lo..hi` Hz.
fn range_mean(levels: &[f32; BARS], (lo, hi): (f32, f32)) -> f32 {
    let (mut sum, mut n) = (0.0, 0);
    for (i, l) in levels.iter().enumerate() {
        let centre = (band_edge(i) * band_edge(i + 1)).sqrt();
        if (lo..hi).contains(&centre) {
            sum += l;
            n += 1;
        }
    }
    if n == 0 { 0.0 } else { sum / n as f32 }
}

/// Number of low bars used for spectral flux (those entirely below FLUX_MAX_HZ).
fn flux_bands() -> usize {
    (0..BARS).take_while(|&i| band_edge(i + 1) <= FLUX_MAX_HZ).count()
}

/// Onset detector: positive spectral flux on the low bars against an adaptive
/// threshold over the last second, with a refractory period.
pub struct BeatDetector {
    /// 1.0 on an onset, falling linearly to 0 over BEAT_DECAY_S.
    pub value: f32,
    prev: [f32; BARS],
    flux: [f32; FLUX_HISTORY],
    dts: [f32; FLUX_HISTORY],
    head: usize,
    len: usize,
    since_beat: f32,
}

impl Default for BeatDetector {
    fn default() -> Self {
        Self {
            value: 0.0,
            prev: [0.0; BARS],
            flux: [0.0; FLUX_HISTORY],
            dts: [0.0; FLUX_HISTORY],
            head: 0,
            len: 0,
            since_beat: f32::INFINITY,
        }
    }
}

impl BeatDetector {
    /// Feed one frame of (unsmoothed) bar levels. Returns true on an onset.
    pub fn update(&mut self, levels: &[f32; BARS], dt: f32) -> bool {
        let n = flux_bands();
        let rise: f32 = (0..n).map(|i| (levels[i] - self.prev[i]).max(0.0)).sum();
        let flux = rise / n as f32;
        self.prev = *levels;

        let (mean, sd) = self.stats();
        self.since_beat += dt;
        self.value = (self.value - dt / BEAT_DECAY_S).max(0.0);
        let onset =
            flux > FLUX_FLOOR && flux > mean + FLUX_K * sd && self.since_beat >= REFRACTORY_S;
        if onset {
            self.value = 1.0;
            self.since_beat = 0.0;
        }

        self.flux[self.head] = flux;
        self.dts[self.head] = dt;
        self.head = (self.head + 1) % FLUX_HISTORY;
        self.len = (self.len + 1).min(FLUX_HISTORY);
        onset
    }

    /// Mean and stddev of the flux values covering the last FLUX_WINDOW_S.
    fn stats(&self) -> (f32, f32) {
        let (mut sum, mut sq, mut t, mut n) = (0.0, 0.0, 0.0, 0);
        for k in 1..=self.len {
            let i = (self.head + FLUX_HISTORY - k) % FLUX_HISTORY;
            sum += self.flux[i];
            sq += self.flux[i] * self.flux[i];
            n += 1;
            t += self.dts[i];
            if t >= FLUX_WINDOW_S {
                break;
            }
        }
        if n == 0 {
            return (0.0, 0.0);
        }
        let mean = sum / n as f32;
        (mean, (sq / n as f32 - mean * mean).max(0.0).sqrt())
    }
}

/// Linear magnitude -> 0..1 using the DB_FLOOR..DB_CEIL range.
pub fn db_to_unit(mag: f32) -> f32 {
    let db = 20.0 * mag.max(1e-12).log10();
    ((db - DB_FLOOR) / (DB_CEIL - DB_FLOOR)).clamp(0.0, 1.0)
}

/// Lower edge frequency of band `i` (band `BARS` gives the top edge).
pub fn band_edge(i: usize) -> f32 {
    MIN_HZ * (MAX_HZ / MIN_HZ).powf(i as f32 / BARS as f32)
}

/// Log-spaced bands: the max bin magnitude inside each band. Bands narrower than
/// one bin (the low end) instead interpolate the spectrum at their centre frequency.
pub fn band_levels(mags: &[f32; BINS], sample_rate: f32, out: &mut [f32; BARS]) {
    let bin_hz = sample_rate.max(1.0) / FFT_SIZE as f32;
    for (i, o) in out.iter_mut().enumerate() {
        let lo = band_edge(i) / bin_hz;
        let hi = band_edge(i + 1) / bin_hz;
        let first = lo.ceil() as usize;
        let last = (hi.floor() as usize).min(BINS - 1);
        let mag = if first <= last {
            mags[first..=last].iter().fold(0.0f32, |a, &b| a.max(b))
        } else {
            let centre = (lo * hi).sqrt().min((BINS - 1) as f32);
            let k = centre.floor() as usize;
            let t = centre - k as f32;
            let next = mags[(k + 1).min(BINS - 1)];
            mags[k] * (1.0 - t) + next * t
        };
        *o = db_to_unit(mag);
    }
}

/// One-pole smoothing with a fast attack and slower decay, frame-rate independent.
pub fn smooth(current: f32, target: f32, dt: f32) -> f32 {
    let tau = if target > current { ATTACK_TAU } else { DECAY_TAU };
    current + (target - current) * (1.0 - (-dt / tau).exp())
}

/// Peak-hold that falls linearly but never below the bar.
pub fn fall_peak(peak: f32, bar: f32, dt: f32) -> f32 {
    (peak - PEAK_FALL_PER_S * dt).max(bar)
}

/// RMS loudness of the raw input mapped from ENERGY_FLOOR_DB..0 dBFS to 0..1.
pub fn rms_level(samples: &[f32]) -> f32 {
    let sum: f32 = samples.iter().filter(|s| s.is_finite()).map(|s| s * s).sum();
    let rms = (sum / samples.len() as f32).sqrt();
    let db = 20.0 * rms.max(1e-12).log10();
    ((db - ENERGY_FLOOR_DB) / -ENERGY_FLOOR_DB).clamp(0.0, 1.0)
}

// --- WASM ABI -----------------------------------------------------------------
// wasm32-unknown-unknown is single-threaded, so one global analyzer is fine.

static mut ANALYZER: Option<Analyzer> = None;

fn analyzer() -> &'static mut Analyzer {
    // SAFETY: single-threaded target; JS never re-enters while we run.
    let p = &raw mut ANALYZER;
    unsafe { (*p).get_or_insert_with(Analyzer::new) }
}

#[unsafe(no_mangle)]
pub extern "C" fn input_ptr() -> *mut f32 {
    analyzer().input.as_mut_ptr()
}

#[unsafe(no_mangle)]
pub extern "C" fn output_ptr() -> *const f32 {
    analyzer().bars.as_ptr()
}

#[unsafe(no_mangle)]
pub extern "C" fn peaks_ptr() -> *const f32 {
    analyzer().peaks.as_ptr()
}

#[unsafe(no_mangle)]
pub extern "C" fn energy() -> f32 {
    analyzer().energy
}

#[unsafe(no_mangle)]
pub extern "C" fn bass() -> f32 {
    analyzer().bass
}

#[unsafe(no_mangle)]
pub extern "C" fn mid() -> f32 {
    analyzer().mid
}

#[unsafe(no_mangle)]
pub extern "C" fn treble() -> f32 {
    analyzer().treble
}

#[unsafe(no_mangle)]
pub extern "C" fn beat() -> f32 {
    analyzer().beat.value
}

#[unsafe(no_mangle)]
pub extern "C" fn waveform_ptr() -> *const f32 {
    analyzer().waveform.as_ptr()
}

#[unsafe(no_mangle)]
pub extern "C" fn process(sample_rate: f32, dt_seconds: f32) {
    analyzer().process(sample_rate, dt_seconds);
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 48_000.0;

    fn sine(a: &mut Analyzer, hz: f32, amp: f32) {
        for (i, s) in a.input.iter_mut().enumerate() {
            *s = amp * (2.0 * std::f32::consts::PI * hz * i as f32 / SR).sin();
        }
    }

    fn settle(a: &mut Analyzer) {
        for _ in 0..60 {
            a.process(SR, 1.0 / 60.0);
        }
    }

    fn band_of(hz: f32) -> usize {
        (0..BARS).find(|&i| band_edge(i + 1) > hz).unwrap()
    }

    #[test]
    fn silence_is_zero() {
        let mut a = Analyzer::new();
        settle(&mut a);
        assert!(a.bars.iter().chain(&a.peaks).all(|&v| v < 1e-6));
        assert!(a.energy < 1e-6);
    }

    #[test]
    fn sine_1khz_peaks_in_its_band() {
        let mut a = Analyzer::new();
        sine(&mut a, 1000.0, 0.5);
        settle(&mut a);
        let max = (0..BARS).max_by(|&x, &y| a.bars[x].total_cmp(&a.bars[y])).unwrap();
        assert_eq!(max, band_of(1000.0));
        assert!(a.bars[max] > 0.9, "{}", a.bars[max]);
        assert!(a.bars[band_of(100.0)] < 0.2, "{}", a.bars[band_of(100.0)]);
        assert!(a.bars[band_of(10_000.0)] < 0.2);
    }

    #[test]
    fn low_bands_get_values() {
        // 40 Hz is far below one bin's width per band; interpolation must still see it.
        let mut a = Analyzer::new();
        sine(&mut a, 40.0, 0.5);
        settle(&mut a);
        assert!(a.bars[band_of(40.0)] > 0.7, "{}", a.bars[band_of(40.0)]);
    }

    #[test]
    fn peaks_fall_after_signal_stops() {
        let mut a = Analyzer::new();
        sine(&mut a, 1000.0, 0.5);
        settle(&mut a);
        let b = band_of(1000.0);
        let before = a.peaks[b];
        a.input = [0.0; FFT_SIZE];
        a.process(SR, 0.1);
        let mid = a.peaks[b];
        a.process(SR, 0.1);
        assert!(before > mid && mid > a.peaks[b]);
        assert!(a.peaks[b] >= a.bars[b]);
    }

    #[test]
    fn energy_rises_with_loudness() {
        let mut e = Vec::new();
        for amp in [0.01, 0.1, 0.8] {
            let mut a = Analyzer::new();
            sine(&mut a, 440.0, amp);
            settle(&mut a);
            e.push(a.energy);
        }
        assert!(e[0] < e[1] && e[1] < e[2], "{e:?}");
    }

    #[test]
    fn garbage_input_is_safe() {
        let mut a = Analyzer::new();
        a.input[3] = f32::NAN;
        a.input[4] = f32::INFINITY;
        a.process(0.0, f32::NAN);
        a.process(SR, 1.0 / 60.0);
        assert!(a.bars.iter().all(|v| v.is_finite()) && a.energy.is_finite());
    }

    /// Runs `signal(t)` (seconds) through the analyzer the way the browser does: at
    /// 60 fps, each frame sees the most recent FFT_SIZE samples. Returns the times
    /// (s) of frames where an onset fired.
    fn onsets(seconds: f32, signal: impl Fn(f32) -> f32) -> Vec<f32> {
        let mut a = Analyzer::new();
        let dt = 1.0 / 60.0;
        let mut out = Vec::new();
        for f in 1..=(seconds / dt) as usize {
            let end = f as f32 * dt;
            for (i, s) in a.input.iter_mut().enumerate() {
                let t = end - (FFT_SIZE - i) as f32 / SR;
                *s = if t < 0.0 { 0.0 } else { signal(t) };
            }
            a.process(SR, dt);
            if a.beat.value == 1.0 {
                out.push(end);
            }
        }
        out
    }

    fn kick(t: f32) -> f32 {
        let since = t % 0.5;
        0.8 * (2.0 * std::f32::consts::PI * 60.0 * since).sin() * (-since / 0.06).exp()
    }

    #[test]
    fn kick_pattern_gives_beats_on_time() {
        let beats = onsets(4.0, kick);
        let kicks: Vec<f32> = (0..8).map(|k| k as f32 * 0.5).collect();
        assert_eq!(beats.len(), kicks.len(), "{beats:?}");
        for (b, k) in beats.iter().zip(&kicks) {
            assert!((0.0..0.07).contains(&(b - k)), "beat {b} for kick {k}: {beats:?}");
        }
    }

    #[test]
    fn kicks_over_a_steady_pad_still_detected() {
        let pad = |t: f32| 0.2 * (2.0 * std::f32::consts::PI * 330.0 * t).sin();
        let beats = onsets(4.0, |t| kick(t) + pad(t));
        assert_eq!(beats.len(), 8, "{beats:?}");
    }

    #[test]
    fn steady_tone_gives_no_beats() {
        // The tone's own start (silence -> sound) is a real onset; nothing after it.
        for hz in [60.0, 220.0, 1000.0] {
            let beats = onsets(4.0, |t| 0.5 * (2.0 * std::f32::consts::PI * hz * t).sin());
            assert!(beats.iter().all(|&b| b < 0.1), "{hz} Hz: {beats:?}");
        }
    }

    #[test]
    fn beat_decays_over_250ms() {
        let mut d = BeatDetector::default();
        let mut loud = [0.0; BARS];
        assert!(!d.update(&loud, 0.1));
        loud[..10].fill(0.9);
        assert!(d.update(&loud, 0.05));
        assert_eq!(d.value, 1.0);
        d.update(&loud, 0.125);
        assert!((d.value - 0.5).abs() < 1e-4, "{}", d.value);
        d.update(&loud, 0.2);
        assert_eq!(d.value, 0.0);
    }

    #[test]
    fn bands_and_waveform() {
        let mut a = Analyzer::new();
        sine(&mut a, 80.0, 0.5);
        settle(&mut a);
        assert!(a.bass > a.mid && a.mid > a.treble, "{} {} {}", a.bass, a.mid, a.treble);
        sine(&mut a, 5000.0, 0.5);
        settle(&mut a);
        assert!(a.treble > a.bass, "{} {}", a.treble, a.bass);
        // 80 Hz at 48 kHz: 8-sample means barely attenuate it.
        sine(&mut a, 80.0, 0.5);
        a.process(SR, 1.0 / 60.0);
        let max = a.waveform.iter().fold(0.0f32, |m, v| m.max(v.abs()));
        assert!((0.45..=0.5).contains(&max), "{max}");
    }
}
