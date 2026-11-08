//! Cartoon smear physics for Cloudberry, exported as a raw WASM ABI (no wasm-bindgen).
//!
//! Bodies: JS writes `[x, y]` per body at `bodies_ptr()`, calls `step(n, dt)`, then reads
//! `[stretch, angle, blur, speed]` per body at `smear_ptr()`.
//! Rain: `rain_spawn` adds particles, `rain_step` moves them and returns the live count;
//! the first `live` particles at `rain_ptr()` are `[x, y, vx, vy, rot, size, life]`.
//!
//! Everything lives in fixed static buffers: nothing allocates, ever.
//! `web/src/lib/motion/smear.ts` mirrors this math for its JS fallback: keep them in sync.

use std::f32::consts::TAU;

pub const MAX_BODIES: usize = 256;
pub const MAX_RAIN: usize = 256;
pub const RAIN_STRIDE: usize = 7;

/// Velocity smoothing time constant (s): first-order critically damped, no overshoot.
pub const VEL_TAU: f32 = 0.03;
/// Below this speed (px/s) nothing smears, so slow drifts stay crisp.
pub const SPEED_DEADZONE: f32 = 250.0;
pub const STRETCH_PER_SPEED: f32 = 1.0 / 2000.0;
pub const MAX_STRETCH: f32 = 1.5;
/// Blur starts above this stretch and grows `BLUR_PER_STRETCH` px per unit.
pub const BLUR_START: f32 = 0.35;
pub const BLUR_PER_STRETCH: f32 = 6.0;
/// Longer frames (tab in background) are clamped so one hitch is not a huge velocity spike.
pub const MAX_DT: f32 = 0.1;

pub const GRAVITY: f32 = 1500.0;
pub const AIR_DRAG: f32 = 0.6;
pub const WOBBLE_ACCEL: f32 = 140.0;
pub const WOBBLE_FREQ: f32 = 5.0;
pub const RESTITUTION: f32 = 0.45;
pub const FLOOR_FRICTION: f32 = 0.75;
/// Seconds a berry takes to fade out after its first bounce.
pub const FADE_TIME: f32 = 0.9;

// --- pure physics ----------------------------------------------------------------

pub fn smooth_velocity(v: f32, raw: f32, dt: f32) -> f32 {
    v + (raw - v) * (1.0 - (-dt / VEL_TAU).exp())
}

pub fn stretch_for(speed: f32) -> f32 {
    ((speed - SPEED_DEADZONE) * STRETCH_PER_SPEED).clamp(0.0, MAX_STRETCH)
}

pub fn blur_for(stretch: f32) -> f32 {
    ((stretch - BLUR_START) * BLUR_PER_STRETCH).max(0.0)
}

/// Returns `Some(dt)` clamped to `MAX_DT`, or `None` for zero/negative/NaN.
fn usable_dt(dt: f32) -> Option<f32> {
    (dt > 0.0).then(|| dt.min(MAX_DT))
}

#[derive(Clone, Copy)]
struct Body {
    px: f32,
    py: f32,
    vx: f32,
    vy: f32,
    angle: f32,
    seen: bool,
}

impl Body {
    const NEW: Body = Body {
        px: 0.0,
        py: 0.0,
        vx: 0.0,
        vy: 0.0,
        angle: 0.0,
        seen: false,
    };

    /// Advances one body; returns `[stretch, angle, blur, speed]`.
    fn step(&mut self, x: f32, y: f32, dt: f32) -> [f32; 4] {
        if !(x.is_finite() && y.is_finite()) {
            *self = Body::NEW;
            return [0.0; 4];
        }
        if self.seen {
            self.vx = smooth_velocity(self.vx, (x - self.px) / dt, dt);
            self.vy = smooth_velocity(self.vy, (y - self.py) / dt, dt);
        }
        (self.px, self.py, self.seen) = (x, y, true);
        let speed = self.vx.hypot(self.vy);
        // Hold the last direction while (nearly) still so a decaying smear never spins.
        if speed > 1.0 {
            self.angle = self.vy.atan2(self.vx);
        }
        let s = stretch_for(speed);
        [s, self.angle, blur_for(s), speed]
    }
}

pub struct Smear {
    pub bodies: [[f32; 2]; MAX_BODIES],
    pub out: [[f32; 4]; MAX_BODIES],
    state: [Body; MAX_BODIES],
}

impl Default for Smear {
    fn default() -> Self {
        Self::new()
    }
}

impl Smear {
    pub const fn new() -> Self {
        Self {
            bodies: [[0.0; 2]; MAX_BODIES],
            out: [[0.0; 4]; MAX_BODIES],
            state: [Body::NEW; MAX_BODIES],
        }
    }

    pub fn step(&mut self, n: usize, dt: f32) {
        let Some(dt) = usable_dt(dt) else { return };
        let bodies = self.bodies.iter().zip(&mut self.out).zip(&mut self.state);
        for ((&[x, y], out), body) in bodies.take(n) {
            *out = body.step(x, y, dt);
        }
    }

    pub fn reset(&mut self, i: usize) {
        if let Some(b) = self.state.get_mut(i) {
            *b = Body::NEW;
        }
        if let Some(out) = self.out.get_mut(i) {
            *out = [0.0; 4];
        }
    }
}

/// xorshift32: tiny, deterministic, good enough for confetti.
fn next_rand(state: &mut u32) -> f32 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    *state = x;
    (x >> 8) as f32 / (1u32 << 24) as f32
}

pub struct Rain {
    /// `[x, y, vx, vy, rot, size, life]` per particle; the first `live` are valid.
    pub p: [[f32; RAIN_STRIDE]; MAX_RAIN],
    /// `[spin (rad/s), wobble phase, bounced (0/1)]` per particle.
    aux: [[f32; 3]; MAX_RAIN],
    pub live: usize,
    rng: u32,
}

impl Default for Rain {
    fn default() -> Self {
        Self::new()
    }
}

impl Rain {
    pub const fn new() -> Self {
        Self {
            p: [[0.0; RAIN_STRIDE]; MAX_RAIN],
            aux: [[0.0; 3]; MAX_RAIN],
            live: 0,
            rng: 0,
        }
    }

    /// Adds up to `count` berries just above the top edge; the total is capped at `MAX_RAIN`.
    pub fn spawn(&mut self, count: usize, width: f32, seed: u32) {
        self.rng = (seed ^ 0x9E37_79B9).max(1);
        let width = if width.is_finite() {
            width.max(0.0)
        } else {
            0.0
        };
        let rng = &mut self.rng;
        let fresh = self.p.iter_mut().zip(&mut self.aux).skip(self.live);
        let mut added = 0;
        for (p, aux) in fresh.take(count) {
            let size = 14.0 + 14.0 * next_rand(rng);
            *p = [
                next_rand(rng) * width,
                -size - 160.0 * next_rand(rng),
                (next_rand(rng) - 0.5) * 160.0,
                next_rand(rng) * 220.0,
                next_rand(rng) * TAU,
                size,
                1.0,
            ];
            *aux = [(next_rand(rng) - 0.5) * 6.0, next_rand(rng) * TAU, 0.0];
            added += 1;
        }
        self.live += added;
    }

    pub fn step(&mut self, dt: f32, width: f32, height: f32) -> usize {
        let Some(dt) = usable_dt(dt) else {
            return self.live;
        };
        if !(width.is_finite() && height.is_finite()) {
            return self.live;
        }
        let drag = (-AIR_DRAG * dt).exp();
        for (p, aux) in self.p.iter_mut().zip(&mut self.aux).take(self.live) {
            let [x, y, vx, vy, rot, size, life] = p;
            let [spin, phase, bounced] = aux;
            *phase += WOBBLE_FREQ * dt;
            *vx = (*vx + phase.sin() * WOBBLE_ACCEL * dt) * drag;
            *vy = (*vy + GRAVITY * dt) * drag;
            *x += *vx * dt;
            *y += *vy * dt;
            *rot += *spin * dt;
            let r = *size * 0.5;
            if *x < r || *x > width - r {
                // min/max rather than clamp: clamp panics when the walls cross on tiny widths.
                *x = x.max(r.min(width * 0.5)).min((width - r).max(width * 0.5));
                *vx = -*vx * RESTITUTION;
            }
            if *y > height - r {
                *y = height - r;
                *vy = -vy.abs() * RESTITUTION;
                *vx *= FLOOR_FRICTION;
                *spin *= FLOOR_FRICTION;
                *bounced = 1.0;
            }
            if *bounced > 0.0 {
                *life -= dt / FADE_TIME;
            }
            if !(x.is_finite() && y.is_finite()) {
                *life = 0.0;
            }
        }
        self.compact();
        self.live
    }

    /// Drops dead particles, keeping the live ones packed at the front.
    fn compact(&mut self) {
        let mut kept = 0;
        for i in 0..self.live {
            // `get` instead of indexing keeps panic code out of the wasm.
            let (Some(&p), Some(&aux)) = (self.p.get(i), self.aux.get(i)) else {
                break;
            };
            if p[6] > 0.0 {
                if let (Some(dp), Some(da)) = (self.p.get_mut(kept), self.aux.get_mut(kept)) {
                    (*dp, *da) = (p, aux);
                }
                kept += 1;
            }
        }
        self.live = kept;
    }
}

// --- WASM ABI -----------------------------------------------------------------
// wasm32-unknown-unknown is single-threaded, so plain globals are fine.

static mut SMEAR: Smear = Smear::new();
static mut RAIN: Rain = Rain::new();

fn smear() -> &'static mut Smear {
    // SAFETY: single-threaded target; JS never re-enters while we run.
    let p = &raw mut SMEAR;
    unsafe { &mut *p }
}

fn rain() -> &'static mut Rain {
    // SAFETY: as above.
    let p = &raw mut RAIN;
    unsafe { &mut *p }
}

#[unsafe(no_mangle)]
pub extern "C" fn bodies_ptr() -> *mut f32 {
    smear().bodies.as_mut_ptr().cast()
}

#[unsafe(no_mangle)]
pub extern "C" fn smear_ptr() -> *const f32 {
    smear().out.as_ptr().cast()
}

#[unsafe(no_mangle)]
pub extern "C" fn step(n_bodies: u32, dt: f32) {
    smear().step(n_bodies as usize, dt);
}

#[unsafe(no_mangle)]
pub extern "C" fn reset(i: u32) {
    smear().reset(i as usize);
}

#[unsafe(no_mangle)]
pub extern "C" fn rain_spawn(count: u32, width: f32, seed: u32) {
    rain().spawn(count as usize, width, seed);
}

#[unsafe(no_mangle)]
pub extern "C" fn rain_step(dt: f32, width: f32, height: f32) -> u32 {
    rain().step(dt, width, height) as u32
}

#[unsafe(no_mangle)]
pub extern "C" fn rain_ptr() -> *const f32 {
    rain().p.as_ptr().cast()
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f32 = 1.0 / 60.0;

    /// Moves body 0 by (dx, dy) per frame for `frames` frames; returns the last output.
    fn run(s: &mut Smear, dx: f32, dy: f32, frames: usize) -> [f32; 4] {
        for _ in 0..frames {
            s.bodies[0][0] += dx;
            s.bodies[0][1] += dy;
            s.step(1, DT);
        }
        s.out[0]
    }

    #[test]
    fn still_body_has_no_stretch() {
        let mut s = Smear::new();
        s.bodies[0] = [100.0, 50.0];
        let [stretch, _, blur, speed] = run(&mut s, 0.0, 0.0, 30);
        assert_eq!((stretch, blur, speed), (0.0, 0.0, 0.0));
    }

    #[test]
    fn fast_constant_velocity_stretches_along_motion() {
        let mut s = Smear::new();
        let [stretch, angle, blur, speed] = run(&mut s, 25.0, 0.0, 20); // 1500 px/s
        assert!((speed - 1500.0).abs() < 5.0, "speed {speed}");
        assert!(stretch > 0.5, "stretch {stretch}");
        assert!(angle.abs() < 1e-3, "angle {angle}");
        assert!(blur > 0.0);
    }

    #[test]
    fn diagonal_motion_angle() {
        let mut s = Smear::new();
        let [_, angle, _, _] = run(&mut s, -20.0, 20.0, 20); // down-left
        assert!(
            (angle - 0.75 * std::f32::consts::PI).abs() < 1e-3,
            "angle {angle}"
        );
    }

    #[test]
    fn slow_drift_stays_crisp() {
        let mut s = Smear::new();
        let [stretch, ..] = run(&mut s, 3.0, 0.0, 30); // 180 px/s
        assert_eq!(stretch, 0.0);
    }

    #[test]
    fn stretch_decays_within_150ms_of_stopping() {
        let mut s = Smear::new();
        run(&mut s, 50.0, 0.0, 20); // 3000 px/s, max stretch
        let mut prev = s.out[0][0];
        for _ in 0..9 {
            s.step(1, DT); // 9 frames = 150 ms
            assert!(s.out[0][0] <= prev, "decays monotonically");
            prev = s.out[0][0];
        }
        assert!(s.out[0][0] < 0.01, "stretch {}", s.out[0][0]);
        assert_eq!(s.out[0][1], 0.0, "angle held while stopping");
    }

    #[test]
    fn stretch_is_capped() {
        let mut s = Smear::new();
        let [stretch, ..] = run(&mut s, 1000.0, 0.0, 10);
        assert_eq!(stretch, MAX_STRETCH);
    }

    #[test]
    fn reset_forgets_history() {
        let mut s = Smear::new();
        run(&mut s, 40.0, 0.0, 10);
        s.reset(0);
        assert_eq!(s.out[0], [0.0; 4]);
        s.bodies[0][0] = 99_999.0; // teleport
        s.step(1, DT);
        assert_eq!(s.out[0][0], 0.0, "no smear from a teleport after reset");
    }

    #[test]
    fn bad_input_is_safe() {
        let mut s = Smear::new();
        s.bodies[0][0] = f32::NAN;
        s.step(1, DT);
        s.step(1, f32::NAN);
        s.step(1, 0.0);
        s.step(10_000, DT);
        s.reset(10_000);
        assert!(s.out.as_flattened().iter().all(|v| v.is_finite()));
    }

    #[test]
    fn only_n_bodies_are_stepped() {
        let mut s = Smear::new();
        s.bodies[1][0] = 50.0;
        s.step(1, DT);
        s.bodies[1][0] = 500.0;
        s.step(1, DT);
        assert_eq!(s.out[1], [0.0; 4]);
    }

    #[test]
    fn rain_falls_and_stays_in_bounds() {
        let (w, h) = (800.0, 600.0);
        let mut r = Rain::new();
        r.spawn(100, w, 7);
        let y0: f32 = r.p[..r.live].iter().map(|p| p[1]).sum::<f32>() / r.live as f32;
        r.step(0.2, w, h);
        let y1: f32 = r.p[..r.live].iter().map(|p| p[1]).sum::<f32>() / r.live as f32;
        assert!(y1 > y0, "falls: {y0} -> {y1}");
        for _ in 0..120 {
            r.step(DT, w, h);
            for p in &r.p[..r.live] {
                assert!((0.0..=w).contains(&p[0]), "x {}", p[0]);
                assert!(p[1] <= h, "y {}", p[1]);
                assert!(p[6] > 0.0, "only live particles");
            }
        }
    }

    #[test]
    fn rain_dies_out() {
        let mut r = Rain::new();
        r.spawn(50, 400.0, 1);
        let mut live = r.live;
        for _ in 0..600 {
            live = r.step(DT, 400.0, 300.0);
        }
        assert_eq!(live, 0);
    }

    #[test]
    fn rain_caps_at_256() {
        let mut r = Rain::new();
        r.spawn(200, 400.0, 1);
        r.spawn(200, 400.0, 2);
        assert_eq!(r.live, MAX_RAIN);
        assert_eq!(r.step(DT, 400.0, 300.0), MAX_RAIN);
    }

    #[test]
    fn rain_is_deterministic_per_seed() {
        let (mut a, mut b) = (Rain::new(), Rain::new());
        a.spawn(10, 400.0, 42);
        b.spawn(10, 400.0, 42);
        assert_eq!(a.p, b.p);
    }
}
