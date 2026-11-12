//! Wegwijs: stap-voor-stap navigatie voor Mira-glasses.
//! De telefoon berekent routes, volgt de gps en stuurt compacte tekstregels;
//! de bril tekent zelf een vectorkaart (looprichting boven), het pijlpaneel,
//! de stappenlijst en de restaurant-keuzekaarten.
#![no_std]
#![allow(static_mut_refs)]
#[path = "../Shared/companion.rs"] mod companion;
#[path = "../Shared/content.rs"] mod content;
#[path = "../Shared/replica_ui.rs"] mod ui;
use ui::*;

app_icon!(content::app_icon::Icon::new()
    .line(16,3,27,27).line(27,27,16,21).line(16,21,5,27).line(5,27,16,3).0);

#[link(wasm_import_module = "mira")]
extern "C" { fn device_status(out: *mut u8, capacity: i32) -> i32; }
#[panic_handler] fn panic(_: &core::panic::PanicInfo) -> ! { loop {} }

const APP: &[u8] = b"Wegwijs";
static mut BRIDGE: companion::Companion<4096, 96> = companion::Companion::new(APP);

// ---- Message intake ---------------------------------------------------------
const STATE_CAP: usize = 3600;
const MAX_LINES: usize = 200;
static mut STATE: [u8; STATE_CAP] = [0; STATE_CAP];
static mut STATE_LEN: usize = 0;
static mut LINES: [(u16, u16); MAX_LINES] = [(0, 0); MAX_LINES];
static mut NLINES: usize = 0;

// ---- Route (frame coordinates in meters: x east, y north) --------------------
const MAX_PTS: usize = 240;
static mut RX: [i32; MAX_PTS] = [0; MAX_PTS];
static mut RY: [i32; MAX_PTS] = [0; MAX_PTS];
static mut NPTS: usize = 0;
const MAX_STEPS: usize = 40;
#[derive(Clone, Copy)]
struct Step { at: u16, code: u8, exit: u8, dist: u16, tlen: u8, text: [u8; 40] }
const NO_STEP: Step = Step { at: 0, code: 0, exit: 0, dist: 0, tlen: 0, text: [0; 40] };
static mut STEPS: [Step; MAX_STEPS] = [NO_STEP; MAX_STEPS];
static mut NSTEPS: usize = 0;
static mut TOTAL_M: i32 = 0;
// Nearby streets as context lines.
const MAX_SPTS: usize = 720;
const MAX_SLINES: usize = 140;
static mut SX: [i32; MAX_SPTS] = [0; MAX_SPTS];
static mut SY: [i32; MAX_SPTS] = [0; MAX_SPTS];
static mut NSPTS: usize = 0;
static mut SSTART: [u16; MAX_SLINES + 1] = [0; MAX_SLINES + 1];
static mut NSLINES: usize = 0;

// ---- Live navigation state (from the `nav` snapshot) -------------------------
static mut NSTATE: u8 = 0; // 0 idle, 1 routing, 2 navigating, 3 arrived, 4 rerouting, 5 no gps, 6 error
static mut STATUS_TXT: [u8; 64] = [0; 64];
static mut STATUS_LEN: u8 = 0;
static mut HAVE_POS: bool = false;
static mut PX: i32 = 0;
static mut PY: i32 = 0;
static mut HDG: i32 = -1;
static mut ACC: i32 = 0;
static mut MOVING: bool = false;
static mut M_CODE: u8 = 0;
static mut M_DIST: i32 = -1;
static mut M_EXIT: u8 = 0;
static mut M_STREET: [u8; 48] = [0; 48];
static mut M_SLEN: u8 = 0;
static mut A_CODE: u8 = 0;
static mut A_STREET: [u8; 32] = [0; 32];
static mut A_SLEN: u8 = 0;
static mut REM_M: i32 = -1;
static mut REM_MIN: i32 = -1;
static mut ETA: [u8; 8] = [0; 8];
static mut ETA_LEN: u8 = 0;
static mut DEST: [u8; 48] = [0; 48];
static mut DEST_LEN: u8 = 0;
static mut STEP_IDX: usize = 0;
static mut SEG_IDX: usize = 0;
static mut MODE: u8 = b'w'; // w walk, b bike, c car
static mut ROTATE: bool = true; // heading-up map when following

// ---- Restaurant chooser --------------------------------------------------------
const MAX_PLACES: usize = 8;
#[derive(Clone, Copy)]
struct Place { r10: u8, open: u8, nlen: u8, ilen: u8, alen: u8, reviews: u16, dist: u16, name: [u8; 40], info: [u8; 40], addr: [u8; 48] }
const NO_PLACE: Place = Place { r10: 0, open: 0, nlen: 0, ilen: 0, alen: 0, reviews: 0, dist: 0, name: [0; 40], info: [0; 40], addr: [0; 48] };
static mut PLACES: [Place; MAX_PLACES] = [NO_PLACE; MAX_PLACES];
static mut NPLACES: usize = 0;
static mut PLACE_SEL: usize = 0;
static mut CHOOSER: bool = false;

// ---- UI state ---------------------------------------------------------------
static mut PAGE: u8 = 0; // 0 navigation, 1 steps
static mut OVERVIEW: bool = false;
static mut OVERVIEW_MS: u32 = 0;
static mut S256: i32 = 512; // current follow-mode scale, px per meter * 256
static mut S256_T: i32 = 512;
static mut OV_S256: i32 = 64;
static mut STEPS_TOP: usize = 0;
static mut DIRTY: bool = true;
static mut ANIM_MS: u32 = 0;
static mut OUT: [u8; 64] = [0; 64];
static mut OUT_LEN: usize = 0;
static mut STATUS: [u8; 10] = [0; 10];
static mut STATUS_MS: u32 = 1000;
static mut PRIMS: i32 = 0;
const MAP_PRIM_CAP: i32 = 340;

// ---- Small helpers ----------------------------------------------------------
fn field<'a>(l: &'a [u8], index: usize) -> &'a [u8] {
    l.split(|b| *b == b'|').nth(index).unwrap_or(&[])
}
fn num(v: &[u8]) -> i32 {
    let (neg, d) = if v.first() == Some(&b'-') { (true, &v[1..]) } else { (false, v) };
    let n = d.iter().take_while(|b| b.is_ascii_digit())
        .fold(0i32, |a, b| a.saturating_mul(10).saturating_add((b - b'0') as i32));
    if neg { -n } else { n }
}
fn copy_into(dst: &mut [u8], src: &[u8]) -> u8 {
    let mut n = src.len().min(dst.len());
    while n > 0 && n < src.len() && (src[n] & 0xc0) == 0x80 { n -= 1; }
    dst[..n].copy_from_slice(&src[..n]);
    n as u8
}
unsafe fn ln(i: usize) -> &'static [u8] {
    let (s, e) = LINES[i];
    &STATE[s as usize..e as usize]
}
unsafe fn queue(msg: &[u8]) {
    let n = msg.len().min(OUT.len());
    OUT[..n].copy_from_slice(&msg[..n]);
    OUT_LEN = n;
}
fn two(out: &mut Buffer, v: u8) { out.add(&[b'0' + v / 10 % 10, b'0' + v % 10]); }
fn bwidth(v: &[u8], font: i32) -> i32 { width(core::str::from_utf8(v).unwrap_or(""), font) }
/// One ellipsized line. `font` is a pixel size: 18, 24, 32 or 40.
unsafe fn t(x: i32, y: i32, w: i32, font: i32, gray: i32, align: i32, v: &[u8]) {
    let h = match font { 18 => 24, 24 => 33, 32 => 47, _ => 57 };
    single(x, y, w, h, gray, font, align, v);
}
unsafe fn wrap(x: i32, y: i32, w: i32, font: i32, gray: i32, align: i32, spacing: i32, max: usize, v: &[u8]) {
    label(x, y, w, spacing * max as i32, gray, font, align, spacing, max, true, false, v);
}

const SIN: [i16; 91] = [0, 71, 143, 214, 286, 357, 428, 499, 570, 641, 711, 782, 852, 921, 991, 1060, 1129, 1198, 1266, 1334, 1401, 1468, 1534, 1600, 1666, 1731, 1796, 1860, 1923, 1986, 2048, 2110, 2171, 2231, 2290, 2349, 2408, 2465, 2522, 2578, 2633, 2687, 2741, 2793, 2845, 2896, 2946, 2996, 3044, 3091, 3138, 3183, 3228, 3271, 3314, 3355, 3396, 3435, 3474, 3511, 3547, 3582, 3617, 3650, 3681, 3712, 3742, 3770, 3798, 3824, 3849, 3873, 3896, 3917, 3937, 3956, 3974, 3991, 4006, 4021, 4034, 4046, 4056, 4065, 4074, 4080, 4086, 4090, 4094, 4095, 4096];
fn sin_q(deg: i32) -> i32 {
    let d = deg.rem_euclid(360);
    match d {
        0..=90 => SIN[d as usize] as i32,
        91..=180 => SIN[(180 - d) as usize] as i32,
        181..=270 => -(SIN[(d - 180) as usize] as i32),
        _ => -(SIN[(360 - d) as usize] as i32),
    }
}
fn cos_q(deg: i32) -> i32 { sin_q(deg + 90) }

// ---- Formatting -------------------------------------------------------------
fn fmt_dist(b: &mut Buffer, m: i32) {
    if m < 0 { b.add(b"--"); return; }
    if m < 950 {
        let r = if m < 100 { (m + 2) / 5 * 5 } else { (m + 5) / 10 * 10 };
        b.num(r as usize); b.add(b" m");
    } else if m < 9950 {
        let tenths = (m + 50) / 100;
        b.num((tenths / 10) as usize); b.add(b","); b.num((tenths % 10) as usize); b.add(b" km");
    } else {
        b.num(((m + 500) / 1000) as usize); b.add(b" km");
    }
}
fn fmt_min(b: &mut Buffer, min: i32) {
    if min < 0 { b.add(b"--"); return; }
    if min < 60 { b.num(min as usize); b.add(b" min"); }
    else { b.num((min / 60) as usize); b.add(b" u "); two(b, (min % 60) as u8); }
}
fn ordinal(b: &mut Buffer, n: u8) { b.num(n as usize); b.add(if n == 1 || n == 8 || n >= 20 { b"ste" } else { b"e" }); }
/// Dutch word for a maneuver code, as the phone defines them.
fn word(b: &mut Buffer, code: u8, exit: u8) {
    b.add(match code {
        b'S' => b"rechtdoor", b'L' => b"linksaf", b'R' => b"rechtsaf",
        b'l' => b"flauw links", b'r' => b"flauw rechts",
        b'H' => b"scherp links", b'h' => b"scherp rechts",
        b'U' | b'u' => b"keer om", b'K' => b"links aanhouden", b'k' => b"rechts aanhouden",
        b'A' => b"bestemming", b'D' => b"vertrek", b'T' => b"trap", b'F' => b"veerboot",
        b'O' => b"rotonde", _ => b"volg de route",
    });
    if code == b'O' && exit > 0 { b.add(b", "); ordinal(b, exit); b.add(b" afslag"); }
}
fn mode_name(mode: u8) -> &'static [u8] {
    match mode { b'c' => b"met de auto", b'b' => b"met de fiets", _ => b"te voet" }
}

// ---- Maneuver arrows -------------------------------------------------------------
const DIRS: [(i32, i32); 8] = [(0, -1000), (707, -707), (1000, 0), (707, 707), (0, 1000), (-707, 707), (-1000, 0), (-707, -707)];
unsafe fn head(px: i32, py: i32, dir: usize, hw: i32, hl: i32, gray: i32) {
    let (ux, uy) = DIRS[dir & 7];
    let (vx, vy) = (-uy, ux);
    fill_triangle(px + ux * hl / 1000, py + uy * hl / 1000,
        px + vx * hw / 1000, py + vy * hw / 1000, px - vx * hw / 1000, py - vy * hw / 1000, gray);
}
/// Draws the maneuver glyph for `code` inside an `s` px box centred on (cx, cy).
unsafe fn arrow(cx: i32, cy: i32, s: i32, code: u8, exit: u8, gray: i32) {
    let mirror = matches!(code, b'R' | b'r' | b'h' | b'u' | b'k');
    let base = match code { b'R' => b'L', b'r' => b'l', b'h' => b'H', b'u' => b'U', b'k' => b'K', c => c };
    let xm = |v: i32| if mirror { 100 - v } else { v };
    let x = |v: i32| cx - s / 2 + xm(v) * s / 100;
    let y = |v: i32| cy - s / 2 + v * s / 100;
    let dm = |d: usize| if mirror { (8 - d) % 8 } else { d };
    let w = (s / 11).max(2);
    let hw = (s * 15 / 100).max(3);
    let hl = (s * 17 / 100).max(4);
    let dim = if gray >= 9 { gray - 6 } else { 2 };
    match base {
        b'L' => {
            line(x(50), y(90), x(50), y(46), w, gray); line(x(50), y(46), x(32), y(46), w, gray);
            head(x(32), y(46), dm(6), hw, hl, gray);
        }
        b'l' => {
            line(x(50), y(90), x(50), y(58), w, gray); line(x(50), y(58), x(31), y(39), w, gray);
            head(x(31), y(39), dm(7), hw, hl, gray);
        }
        b'H' => {
            line(x(50), y(90), x(50), y(38), w, gray); line(x(50), y(38), x(30), y(58), w, gray);
            head(x(30), y(58), dm(5), hw, hl, gray);
        }
        b'U' => {
            line(x(66), y(90), x(66), y(50), w, gray);
            let (mut lx, mut ly) = (x(66), y(50));
            let mut a = 30;
            while a <= 180 {
                let nx = cx - s / 2 + xm(50 + 16 * cos_q(a) / 4096) * s / 100;
                let ny = cy - s / 2 + (50 - 16 * sin_q(a) / 4096) * s / 100;
                line(lx, ly, nx, ny, w, gray); lx = nx; ly = ny; a += 30;
            }
            line(x(34), y(50), x(34), y(66), w, gray);
            head(x(34), y(66), 4, hw, hl, gray);
        }
        b'O' => {
            stroke_circle(x(50), y(50), s * 20 / 100, w, gray);
            line(x(50), y(92), x(50), y(70), w, gray);
            line(x(50), y(30), x(50), y(20), w, gray);
            head(x(50), y(20), 0, hw, hl, gray);
            if exit > 0 && s >= 60 {
                let mut b = Buffer::new(); b.num(exit as usize);
                t(x(50) - 20, y(50) - 11, 40, 18, gray, 1, b.bytes());
            }
        }
        b'K' => {
            line(x(50), y(90), x(50), y(62), w, gray); line(x(50), y(62), x(36), y(46), w, gray);
            line(x(36), y(46), x(36), y(30), w, gray); head(x(36), y(30), 0, hw, hl, gray);
            line(x(50), y(62), x(64), y(46), w / 2 + 1, dim); line(x(64), y(46), x(64), y(32), w / 2 + 1, dim);
        }
        b'A' => {
            line(x(34), y(14), x(34), y(92), w, gray);
            stroke_rect(x(34), y(16), 42 * s / 100, 30 * s / 100, (w / 2).max(1), gray);
            for row in 0..2 { for col in 0..3 {
                if (row + col) % 2 == 0 { fill_rect(x(34 + col * 14), y(16 + row * 15), 14 * s / 100 + 1, 15 * s / 100 + 1, gray); }
            } }
        }
        b'D' => {
            fill_circle(x(50), y(80), (s * 7 / 100).max(3), gray);
            line(x(50), y(70), x(50), y(34), w, gray); head(x(50), y(34), 0, hw, hl, gray);
        }
        b'T' => {
            let pts = [(24, 84), (38, 84), (38, 70), (50, 70), (50, 56), (62, 56), (62, 42), (76, 42)];
            for i in 1..pts.len() { line(x(pts[i - 1].0), y(pts[i - 1].1), x(pts[i].0), y(pts[i].1), (w * 2 / 3).max(2), gray); }
        }
        b'F' => {
            line(x(22), y(66), x(30), y(80), w, gray); line(x(30), y(80), x(70), y(80), w, gray);
            line(x(70), y(80), x(78), y(66), w, gray); line(x(22), y(66), x(78), y(66), w, gray);
            line(x(50), y(34), x(50), y(66), w, gray);
            fill_triangle(x(50), y(36), x(50), y(62), x(72), y(62), gray);
        }
        _ => {
            line(x(50), y(90), x(50), y(34), w, gray); head(x(50), y(34), 0, hw, hl, gray);
        }
    }
}

// ---- Map projection ---------------------------------------------------------
static mut T_C: i64 = 4096;
static mut T_S: i64 = 0;
static mut T_OX: i64 = 0;
static mut T_OY: i64 = 0;
static mut T_CX: i64 = 320;
static mut T_CY: i64 = 240;
static mut T_S256: i64 = 256;
unsafe fn set_view(ox: i32, oy: i32, cx: i32, cy: i32, heading: i32, s256: i32) {
    T_C = cos_q(heading) as i64; T_S = sin_q(heading) as i64;
    T_OX = ox as i64; T_OY = oy as i64; T_CX = cx as i64; T_CY = cy as i64; T_S256 = s256.max(1) as i64;
}
unsafe fn tf(x: i32, y: i32) -> (i32, i32) {
    let dx = x as i64 - T_OX; let dy = y as i64 - T_OY;
    let ex = dx * T_C - dy * T_S; let ey = dx * T_S + dy * T_C;
    let sx = T_CX + ((ex * T_S256) >> 20); let sy = T_CY - ((ey * T_S256) >> 20);
    (sx.clamp(-1_000_000, 1_000_000) as i32, sy.clamp(-1_000_000, 1_000_000) as i32)
}
const CX_MIN: i64 = -64; const CX_MAX: i64 = 704; const CY_MIN: i64 = -64; const CY_MAX: i64 = 544;
fn outcode(x: i64, y: i64) -> u8 {
    (if x < CX_MIN { 1 } else if x > CX_MAX { 2 } else { 0 }) | (if y < CY_MIN { 4 } else if y > CY_MAX { 8 } else { 0 })
}
/// Cohen-Sutherland clip against a box slightly larger than the canvas.
fn clip(mut x0: i64, mut y0: i64, mut x1: i64, mut y1: i64) -> Option<(i32, i32, i32, i32)> {
    let mut c0 = outcode(x0, y0); let mut c1 = outcode(x1, y1);
    for _ in 0..6 {
        if c0 | c1 == 0 { return Some((x0 as i32, y0 as i32, x1 as i32, y1 as i32)); }
        if c0 & c1 != 0 { return None; }
        let c = if c0 != 0 { c0 } else { c1 };
        let (x, y);
        if c & 8 != 0 { x = x0 + (x1 - x0) * (CY_MAX - y0) / (y1 - y0); y = CY_MAX; }
        else if c & 4 != 0 { x = x0 + (x1 - x0) * (CY_MIN - y0) / (y1 - y0); y = CY_MIN; }
        else if c & 2 != 0 { y = y0 + (y1 - y0) * (CX_MAX - x0) / (x1 - x0); x = CX_MAX; }
        else { y = y0 + (y1 - y0) * (CX_MIN - x0) / (x1 - x0); x = CX_MIN; }
        if c == c0 { x0 = x; y0 = y; c0 = outcode(x0, y0); } else { x1 = x; y1 = y; c1 = outcode(x1, y1); }
    }
    None
}
unsafe fn seg(a: (i32, i32), b: (i32, i32), w: i32, gray: i32) -> bool {
    if PRIMS >= MAP_PRIM_CAP { return false; }
    if let Some((x0, y0, x1, y1)) = clip(a.0 as i64, a.1 as i64, b.0 as i64, b.1 as i64) {
        if x0 != x1 || y0 != y1 { line(x0, y0, x1, y1, w, gray); PRIMS += 1; }
    }
    true
}
unsafe fn draw_streets() {
    for k in 0..NSLINES {
        let (s, e) = (SSTART[k] as usize, SSTART[k + 1] as usize);
        let mut prev = tf(SX[s], SY[s]);
        for i in s + 1..e {
            let cur = tf(SX[i], SY[i]);
            if !seg(prev, cur, 2, 4) { return; }
            prev = cur;
        }
    }
}
unsafe fn draw_route() {
    if NPTS < 2 { return; }
    // Travelled part first and dim, so the remaining route stays on top.
    let mut prev = tf(RX[0], RY[0]);
    for i in 1..NPTS {
        let cur = tf(RX[i], RY[i]);
        if i <= SEG_IDX { if !seg(prev, cur, 2, 6) { break; } }
        prev = cur;
    }
    let start = SEG_IDX.min(NPTS - 1);
    prev = tf(RX[start], RY[start]);
    for i in start + 1..NPTS {
        let cur = tf(RX[i], RY[i]);
        if !seg(prev, cur, 5, 15) { break; }
        prev = cur;
    }
    // Next turn and destination markers.
    if STEP_IDX + 1 < NSTEPS {
        let at = (STEPS[STEP_IDX + 1].at as usize).min(NPTS - 1);
        let (x, y) = tf(RX[at], RY[at]);
        if x > -20 && x < 660 && y > -20 && y < 500 { fill_circle(x, y, 7, 15); fill_circle(x, y, 3, 0); }
    }
    let (dx, dy) = tf(RX[NPTS - 1], RY[NPTS - 1]);
    if dx > -20 && dx < 660 && dy > -20 && dy < 500 {
        fill_circle(dx, dy, 11, 0); stroke_circle(dx, dy, 10, 3, 15); fill_circle(dx, dy, 4, 15);
    }
}
/// Position marker: a navigation chevron rotated clockwise by `heading` degrees.
unsafe fn chevron(cx: i32, cy: i32, heading: i32, gray: i32) {
    let c = cos_q(heading); let s = sin_q(heading);
    let rot = |x: i32, y: i32| (cx + (x * c - y * s) / 4096, cy + (x * s + y * c) / 4096);
    let tip = rot(0, -17); let left = rot(-12, 12); let right = rot(12, 12); let notch = rot(0, 5);
    fill_circle(cx, cy, 19, 0);
    fill_triangle(tip.0, tip.1, left.0, left.1, notch.0, notch.1, gray);
    fill_triangle(tip.0, tip.1, notch.0, notch.1, right.0, right.1, gray);
}
unsafe fn position_marker(cx: i32, cy: i32, heading: i32, s256: i32) {
    if ACC > 15 {
        let r = ACC * s256 / 256;
        if r > 22 && r < 320 { stroke_circle(cx, cy, r, 1, 5); }
    }
    if heading >= 0 { chevron(cx, cy, heading, 15); }
    else { fill_circle(cx, cy, 13, 0); fill_circle(cx, cy, 8, 15); stroke_circle(cx, cy, 12, 2, 8); }
}
unsafe fn north_dial(cx: i32, cy: i32, heading: i32) {
    fill_circle(cx, cy, 17, 0); stroke_circle(cx, cy, 16, 1, 6);
    let a = -heading;
    let (sx, sy) = (sin_q(a), cos_q(a));
    let tip = (cx + sx * 12 / 4096, cy - sy * 12 / 4096);
    let tail = (cx - sx * 12 / 4096, cy + sy * 12 / 4096);
    let side = (cy_perp(sy), cx_perp(sx));
    fill_triangle(tip.0, tip.1, cx + side.0, cy + side.1, cx - side.0, cy - side.1, 15);
    fill_triangle(tail.0, tail.1, cx + side.0, cy + side.1, cx - side.0, cy - side.1, 6);
    t(cx - 20, cy + 18, 40, 18, 8, 1, b"N");
}
fn cy_perp(sy: i32) -> i32 { sy * 4 / 4096 }
fn cx_perp(sx: i32) -> i32 { sx * 4 / 4096 }
unsafe fn scale_bar(x: i32, y: i32, s256: i32) {
    const NICE: [i32; 10] = [5, 10, 20, 50, 100, 200, 500, 1000, 2000, 5000];
    let mut pick = NICE[0];
    for n in NICE { if n * s256 / 256 <= 150 { pick = n; } }
    let px = (pick * s256 / 256).max(8);
    line(x, y, x + px, y, 2, 9); line(x, y - 5, x, y + 1, 2, 9); line(x + px, y - 5, x + px, y + 1, 2, 9);
    let mut b = Buffer::new(); fmt_dist(&mut b, pick);
    t(x + 4, y - 28, 160, 18, 9, 0, b.bytes());
}

// ---- Chrome pieces ----------------------------------------------------------
unsafe fn battery(x: i32, y: i32) {
    let pct = STATUS[0];
    if pct > 100 { return; }
    let mut b = Buffer::new(); b.num(pct as usize); b.add(b"%");
    t(x - 120, y - 2, 70, 18, 10, 2, b.bytes());
    stroke_round_rect(x - 44, y, 36, 18, 4, 2, 10);
    fill_rect(x - 8, y + 5, 3, 8, 10);
    let fill = (pct as i32 * 28 / 100).max(2);
    fill_rect(x - 40, y + 4, fill, 10, if pct <= 15 { 15 } else { 11 });
    if STATUS[1] != 0 {
        fill_triangle(x - 24, y + 2, x - 32, y + 10, x - 25, y + 10, 0);
        fill_triangle(x - 27, y + 8, x - 20, y + 8, x - 28, y + 16, 0);
    }
}
unsafe fn clock(b: &mut Buffer) {
    if STATUS[4] != 0 { two(b, STATUS[5]); b.add(b":"); two(b, STATUS[6]); }
}
unsafe fn header(title: &[u8], right: &[u8]) {
    t(28, 20, 220, 32, 15, 0, title);
    t(240, 28, 140, 18, 9, 2, right);
    let mut b = Buffer::new(); clock(&mut b);
    t(400, 26, 80, 24, 10, 2, b.bytes());
    battery(620, 30);
    line(28, 72, 612, 72, 1, 5);
}
unsafe fn phone_ok() -> bool { STATUS[3] != 0 || STATUS[0] > 100 }

// ---- Navigation page --------------------------------------------------------
fn zoom_bounds() -> (i32, i32) {
    unsafe { match MODE { b'c' => (16, 400), b'b' => (48, 800), _ => (112, 1400) } }
}
unsafe fn zoom_target() -> i32 {
    let ahead = if M_DIST > 0 { M_DIST } else { 80 };
    let (lo, hi) = zoom_bounds();
    (215 * 256 / ahead.max(1)).clamp(lo, hi)
}
unsafe fn fit_overview() -> (i32, i32, i32) {
    // Bounding box of the route and the position, fitted left of the panel.
    let (mut x0, mut y0, mut x1, mut y1) = (RX[0], RY[0], RX[0], RY[0]);
    for i in 0..NPTS { x0 = x0.min(RX[i]); x1 = x1.max(RX[i]); y0 = y0.min(RY[i]); y1 = y1.max(RY[i]); }
    if HAVE_POS { x0 = x0.min(PX); x1 = x1.max(PX); y0 = y0.min(PY); y1 = y1.max(PY); }
    let bw = (x1 - x0).max(40) as i64; let bh = (y1 - y0).max(40) as i64;
    let s = (340 * 256 / bw).min(360 * 256 / bh).clamp(2, 1024) as i32;
    ((x0 + x1) / 2, (y0 + y1) / 2, s)
}
unsafe fn panel() {
    let (x0, y0, w, h) = (404, 6, 230, 266);
    fill_round_rect(x0, y0, w, h, 16, 0);
    stroke_round_rect(x0, y0, w, h, 16, 1, 6);
    let cx = x0 + w / 2;
    if M_CODE == 0 {
        t(x0 + 12, y0 + 110, w - 24, 24, 9, 1, b"Volg de route");
        return;
    }
    arrow(cx, y0 + 60, 92, M_CODE, M_EXIT, 15);
    let mut b = Buffer::new();
    if M_CODE == b'A' && M_DIST <= 20 { b.add(b"hier"); }
    else if M_DIST >= 0 && M_DIST <= 12 { b.add(b"nu"); }
    else { fmt_dist(&mut b, M_DIST); }
    t(x0 + 8, y0 + 106, w - 16, 40, 15, 1, b.bytes());
    let mut wd = Buffer::new(); word(&mut wd, M_CODE, M_EXIT);
    t(x0 + 12, y0 + 158, w - 24, 18, 10, 1, wd.bytes());
    if M_SLEN > 0 { wrap(x0 + 14, y0 + 182, w - 28, 24, 15, 1, 28, 2, &M_STREET[..M_SLEN as usize]); }
    if A_CODE != 0 {
        line(x0 + 16, y0 + 238, x0 + w - 16, y0 + 238, 1, 4);
        arrow(x0 + 28, y0 + 254, 24, A_CODE, 0, 11);
        let mut a = Buffer::new(); a.add(b"daarna ");
        word(&mut a, A_CODE, 0);
        if A_SLEN > 0 { a.add(b", "); a.add(&A_STREET[..A_SLEN as usize]); }
        t(x0 + 46, y0 + 244, w - 56, 18, 9, 0, a.bytes());
    }
}
unsafe fn footer_nav() {
    fill_rect(0, 434, 640, 46, 0);
    line(0, 434, 640, 434, 1, 4);
    let mut b = Buffer::new(); clock(&mut b);
    t(430, 446, 70, 18, 10, 2, b.bytes());
    battery(620, 448);
    if !phone_ok() { t(28, 446, 380, 18, 9, 0, b"Telefoon niet verbonden"); return; }
    match NSTATE {
        4 | 5 => {
            let msg: &[u8] = if STATUS_LEN > 0 { &STATUS_TXT[..STATUS_LEN as usize] }
                else if NSTATE == 4 { b"Nieuwe route berekenen..." } else { b"Wachten op gps-signaal..." };
            t(28, 446, 400, 18, 10, 0, msg);
        }
        _ => {
            let mut d = Buffer::new(); fmt_dist(&mut d, REM_M);
            t(28, 442, 130, 24, 15, 0, d.bytes());
            let mut m = Buffer::new(); fmt_min(&mut m, REM_MIN);
            t(150, 442, 110, 24, 12, 0, m.bytes());
            if ETA_LEN > 0 {
                let mut e = Buffer::new(); e.add(b"aankomst "); e.add(&ETA[..ETA_LEN as usize]);
                t(262, 446, 170, 18, 8, 0, e.bytes());
            }
        }
    }
}
unsafe fn nav_page() {
    let heading = if HDG >= 0 { HDG } else { 0 };
    if OVERVIEW {
        let (ox, oy, s) = fit_overview();
        OV_S256 = s;
        set_view(ox, oy, 206, 226, 0, s);
        draw_streets(); draw_route();
        if HAVE_POS { let (px, py) = tf(PX, PY); position_marker(px, py, HDG, s); }
        t(28, 14, 300, 18, 8, 0, b"HELE ROUTE");
        t(28, 36, 340, 18, 7, 0, b"tik om te volgen");
        north_dial(34, 94, 0);
        scale_bar(28, 412, s);
    } else {
        let (cx, cy) = (236, 296);
        let up = if ROTATE { heading } else { 0 };
        set_view(PX, PY, cx, cy, up, S256);
        draw_streets(); draw_route();
        if HAVE_POS { position_marker(cx, cy, if HDG < 0 { -1 } else if ROTATE { 0 } else { HDG }, S256); }
        if HDG >= 0 && ROTATE { north_dial(34, 34, heading); }
        scale_bar(28, 412, S256);
    }
    panel();
    footer_nav();
}

// ---- Steps page -------------------------------------------------------------
unsafe fn steps_page() {
    let mut r = Buffer::new();
    if REM_M >= 0 { r.add(b"nog "); fmt_dist(&mut r, REM_M); }
    header(b"Stappen", r.bytes());
    if NSTEPS == 0 { t(28, 120, 584, 24, 10, 0, b"Nog geen route"); return; }
    let first = (STEP_IDX + 1).min(NSTEPS);
    let total = NSTEPS - first;
    if STEPS_TOP >= total { STEPS_TOP = 0; }
    let mut cum = M_DIST.max(0);
    let mut y = 84;
    let mut shown = 0;
    for j in first..NSTEPS {
        let k = j - first;
        if k >= STEPS_TOP {
            if y + 46 > 420 { break; }
            let s = &STEPS[j];
            let ink = if k == 0 { 15 } else { 11 };
            arrow(46, y + 22, 30, s.code, s.exit, ink);
            let mut d = Buffer::new();
            if s.code == b'A' && cum <= 20 { d.add(b"hier"); } else { fmt_dist(&mut d, cum); }
            t(70, y + 8, 96, 24, if k == 0 { 15 } else { 9 }, 2, d.bytes());
            t(184, y + 8, 428, 24, ink, 0, &s.text[..s.tlen as usize]);
            if shown > 0 { line(70, y - 1, 612, y - 1, 1, 3); }
            y += 46; shown += 1;
        }
        cum = cum.saturating_add(STEPS[j].dist as i32);
    }
    if total > shown {
        let mut b = Buffer::new();
        b.num(STEPS_TOP + 1); b.add(b"-"); b.num(STEPS_TOP + shown); b.add(b" van "); b.num(total); b.add(b", tik voor meer");
        t(28, 440, 584, 18, 7, 2, b.bytes());
    } else {
        t(28, 440, 584, 18, 7, 2, mode_name(MODE));
    }
}

// ---- Other screens ----------------------------------------------------------
unsafe fn big_logo(cx: i32, cy: i32, s: i32, gray: i32) {
    let p = |x: i32, y: i32| (cx + x * s / 32, cy + y * s / 32);
    let (a, b, c, d) = (p(0, -14), p(11, 12), p(0, 6), p(-11, 12));
    fill_triangle(a.0, a.1, b.0, b.1, c.0, c.1, gray);
    fill_triangle(a.0, a.1, c.0, c.1, d.0, d.1, gray - 4);
}
unsafe fn idle_page() {
    big_logo(320, 120, 110, 13);
    t(120, 186, 400, 40, 15, 1, b"Wegwijs");
    let msg: &[u8] = match NSTATE {
        1 => b"Route wordt berekend...",
        5 => b"Wachten op gps-signaal...",
        6 => if STATUS_LEN > 0 { &STATUS_TXT[..STATUS_LEN as usize] } else { b"Route mislukt" },
        _ => b"Kies een bestemming in de telefoon-app",
    };
    wrap(60, 250, 520, 24, 11, 1, 30, 2, msg);
    if NSTATE == 1 && DEST_LEN > 0 {
        let mut b = Buffer::new(); b.add(b"naar "); b.add(&DEST[..DEST_LEN as usize]);
        t(60, 312, 520, 24, 13, 1, b.bytes());
    } else if NSTATE == 0 {
        t(60, 312, 520, 18, 8, 1, b"of zoek een restaurant in de buurt");
    }
    let mut c = Buffer::new(); clock(&mut c);
    t(28, 446, 120, 18, 10, 0, c.bytes());
    battery(620, 448);
    let hint: &[u8] = if !phone_ok() { b"Telefoon niet verbonden" }
        else if HAVE_POS { if ACC > 0 && ACC < 500 { b"gps actief" } else { b"" } } else { b"" };
    t(160, 446, 320, 18, 7, 1, hint);
}
unsafe fn arrived_page() {
    stroke_circle(320, 200, 86, 3, 10);
    line(272, 204, 306, 238, 10, 15); line(306, 238, 372, 160, 10, 15);
    t(120, 302, 400, 40, 15, 1, b"Je bent er!");
    if DEST_LEN > 0 { wrap(60, 356, 520, 24, 11, 1, 28, 2, &DEST[..DEST_LEN as usize]); }
    t(120, 436, 400, 18, 7, 1, b"tik om af te sluiten");
}
unsafe fn chooser_page() {
    let n = NPLACES;
    t(28, 16, 400, 24, 10, 0, b"Restaurants in de buurt");
    let mut c = Buffer::new();
    if PLACE_SEL < n { c.num(PLACE_SEL + 1); c.add(b" van "); c.num(n); }
    t(420, 20, 192, 18, 8, 2, c.bytes());
    stroke_round_rect(28, 54, 584, 352, 18, 1, 6);
    // Edge chevrons as swipe hints.
    if n > 0 {
        fill_triangle(14, 230, 22, 220, 22, 240, 6);
        fill_triangle(626, 230, 618, 220, 618, 240, 6);
    }
    if PLACE_SEL >= n {
        t(60, 150, 520, 32, 15, 1, b"Geen keuze");
        t(60, 206, 520, 24, 10, 1, b"Swipe terug voor de voorstellen");
        t(60, 240, 520, 18, 8, 1, b"of tik om te sluiten");
        return;
    }
    let p = &PLACES[PLACE_SEL];
    wrap(50, 68, 540, 32, 15, 0, 38, 2, &p.name[..p.nlen as usize]);
    // Rating as five dots plus the figure.
    let y = 158;
    if p.r10 > 0 {
        for i in 0..5u8 {
            let x = 58 + i as i32 * 24;
            if p.r10 >= i * 10 + 8 { fill_circle(x, y, 8, 15); }
            else if p.r10 >= i * 10 + 3 { stroke_circle(x, y, 8, 2, 15); fill_circle(x, y, 4, 15); }
            else { stroke_circle(x, y, 8, 1, 6); }
        }
        let mut r = Buffer::new(); r.num((p.r10 / 10) as usize); r.add(b","); r.num((p.r10 % 10) as usize);
        t(184, y - 20, 80, 32, 15, 0, r.bytes());
        if p.reviews > 0 {
            let mut v = Buffer::new(); v.num(p.reviews as usize); v.add(b" beoordelingen");
            t(250, y - 10, 340, 18, 8, 0, v.bytes());
        }
    } else {
        t(50, y - 14, 540, 24, 8, 0, b"Beoordeling onbekend");
    }
    if p.ilen > 0 { t(50, 196, 540, 24, 12, 0, &p.info[..p.ilen as usize]); }
    let mut d = Buffer::new(); fmt_dist(&mut d, p.dist as i32); d.add(b" hemelsbreed");
    t(50, 234, 300, 24, 10, 0, d.bytes());
    match p.open { 1 => t(350, 234, 240, 24, 15, 2, b"nu open"), 2 => t(350, 234, 240, 24, 9, 2, b"gesloten"), _ => {} }
    if p.alen > 0 { wrap(50, 272, 540, 18, 8, 0, 24, 2, &p.addr[..p.alen as usize]); }
    let mut h = Buffer::new(); h.add(b"tik om hiernaartoe te gaan ("); h.add(mode_name(MODE)); h.add(b")");
    t(50, 372, 540, 18, 9, 1, h.bytes());
    t(28, 440, 584, 18, 6, 1, b"swipe voor het volgende voorstel");
}

unsafe fn render() {
    begin();
    PRIMS = 0;
    if CHOOSER { chooser_page(); }
    else if NSTATE == 3 { arrived_page(); }
    else if NPTS < 2 || NSTATE == 6 { idle_page(); }
    else if PAGE == 1 { steps_page(); }
    else { nav_page(); }
    content::cursor(612, 458, false);
    present();
}

// ---- JSON intake ------------------------------------------------------------
fn hex(b: u8) -> Option<u32> {
    match b { b'0'..=b'9' => Some((b - b'0') as u32), b'a'..=b'f' => Some((b - b'a' + 10) as u32),
        b'A'..=b'F' => Some((b - b'A' + 10) as u32), _ => None }
}
fn put_utf8(out: &mut [u8], n: &mut usize, cp: u32) {
    let mut tmp = [0u8; 4];
    let s = char::from_u32(cp).unwrap_or('?').encode_utf8(&mut tmp);
    for b in s.bytes() { if *n < out.len() { out[*n] = b; *n += 1; } }
}
fn find_key(json: &[u8], key: &[u8]) -> Option<usize> {
    if json.len() < key.len() { return None; }
    (0..=json.len() - key.len()).find(|&i| &json[i..i + key.len()] == key).map(|i| i + key.len())
}
/// Copies the JSON string starting at `i` into `out`, unescaping it.
fn unescape(json: &[u8], mut i: usize, out: &mut [u8]) -> usize {
    let mut n = 0;
    while i < json.len() && n < out.len() {
        let b = json[i];
        if b == b'"' { break; }
        if b == b'\\' && i + 1 < json.len() {
            let e = json[i + 1]; i += 2;
            match e {
                b'n' => { out[n] = b'\n'; n += 1; }
                b't' | b'r' | b'b' | b'f' => { out[n] = b' '; n += 1; }
                b'u' if i + 4 <= json.len() => {
                    let mut cp = 0u32; let mut ok = true;
                    for k in 0..4 { match hex(json[i + k]) { Some(v) => cp = cp * 16 + v, None => ok = false } }
                    i += 4;
                    if !ok { continue; }
                    if (0xD800..0xDC00).contains(&cp) && i + 6 <= json.len() && json[i] == b'\\' && json[i + 1] == b'u' {
                        let mut lo = 0u32;
                        for k in 0..4 { lo = lo * 16 + hex(json[i + 2 + k]).unwrap_or(0); }
                        if (0xDC00..0xE000).contains(&lo) { cp = 0x10000 + ((cp - 0xD800) << 10) + (lo - 0xDC00); i += 6; }
                    }
                    if cp < 0x20 { cp = if cp == 0x0a { 0x0a } else { 0x20 }; }
                    put_utf8(out, &mut n, cp);
                }
                _ => { out[n] = e; n += 1; }
            }
            continue;
        }
        out[n] = b; n += 1; i += 1;
    }
    n
}
/// Loads a phone message into STATE and returns its type's first letter.
unsafe fn take(json: &[u8]) -> u8 {
    let mut kind = [0u8; 12];
    let klen = match find_key(json, b"\"type\":\"") { Some(i) => unescape(json, i, &mut kind), None => 0 };
    STATE_LEN = match find_key(json, b"\"d\":\"") { Some(i) => unescape(json, i, &mut STATE), None => 0 };
    NLINES = 0;
    let mut s = 0;
    for j in 0..=STATE_LEN {
        if j == STATE_LEN || STATE[j] == b'\n' {
            if j > s && NLINES < MAX_LINES { LINES[NLINES] = (s as u16, j as u16); NLINES += 1; }
            s = j + 1;
        }
    }
    if klen == 0 { 0 } else { kind[0] }
}
fn b64v(c: u8) -> i32 {
    match c { b'A'..=b'Z' => (c - b'A') as i32, b'a'..=b'z' => (c - b'a') as i32 + 26,
        b'0'..=b'9' => (c - b'0') as i32 + 52, b'-' => 62, b'_' => 63, _ => -1 }
}
/// Delta-encoded point stream: zigzag varints, 5 bits per character plus a continuation bit.
fn decode_points(f: &[u8], xs: &mut [i32], ys: &mut [i32]) -> usize {
    let cap = xs.len().min(ys.len());
    let (mut i, mut n) = (0, 0);
    let (mut x, mut y) = (0i32, 0i32);
    while n < cap && i < f.len() {
        let mut vals = [0i32; 2];
        let mut ok = true;
        for v in vals.iter_mut() {
            let mut acc: u32 = 0; let mut shift = 0;
            loop {
                if i >= f.len() || shift > 30 { ok = false; break; }
                let d = b64v(f[i]); i += 1;
                if d < 0 { ok = false; break; }
                acc |= ((d as u32) & 31) << shift; shift += 5;
                if d & 32 == 0 { break; }
            }
            if !ok { break; }
            *v = ((acc >> 1) as i32) ^ -((acc & 1) as i32);
        }
        if !ok { break; }
        x = x.wrapping_add(vals[0]); y = y.wrapping_add(vals[1]);
        xs[n] = x; ys[n] = y; n += 1;
    }
    n
}

unsafe fn apply_nav() {
    HAVE_POS = false; M_CODE = 0; M_DIST = -1; M_EXIT = 0; M_SLEN = 0; A_CODE = 0; A_SLEN = 0;
    REM_M = -1; REM_MIN = -1; ETA_LEN = 0;
    for i in 0..NLINES {
        let l = ln(i);
        if l.len() < 2 || l[1] != b'|' { continue; }
        let f = &l[2..];
        match l[0] {
            b'T' => { NSTATE = num(field(f, 0)).clamp(0, 6) as u8; STATUS_LEN = copy_into(&mut STATUS_TXT, field(f, 1)); }
            b'S' => { PX = num(field(f, 0)); PY = num(field(f, 1)); HDG = num(field(f, 2)); ACC = num(field(f, 3));
                MOVING = field(f, 4) == b"1"; HAVE_POS = true; }
            b'M' => { M_CODE = field(f, 0).first().copied().unwrap_or(0); M_DIST = num(field(f, 1));
                M_SLEN = copy_into(&mut M_STREET, field(f, 2)); M_EXIT = num(field(f, 3)).clamp(0, 12) as u8; }
            b'A' => { A_CODE = field(f, 0).first().copied().unwrap_or(0); A_SLEN = copy_into(&mut A_STREET, field(f, 1)); }
            b'R' => { REM_M = num(field(f, 0)); REM_MIN = num(field(f, 1)); ETA_LEN = copy_into(&mut ETA, field(f, 2)); }
            b'D' => { DEST_LEN = copy_into(&mut DEST, field(f, 0)); }
            b'P' => { STEP_IDX = num(field(f, 0)).max(0) as usize; SEG_IDX = num(field(f, 1)).max(0) as usize; }
            b'V' => { MODE = field(f, 0).first().copied().unwrap_or(b'w'); ROTATE = field(f, 1) != b"0"; }
            _ => {}
        }
    }
    if HDG > 359 { HDG = -1; }
    if NSTATE != 3 && NSTATE != 4 && NSTATE != 2 && NSTATE != 5 { OVERVIEW = false; }
}
unsafe fn apply_route() {
    NPTS = 0; NSTEPS = 0; STEPS_TOP = 0; PAGE = 0;
    let mut overview = false;
    for i in 0..NLINES {
        let l = ln(i);
        if l.len() < 2 || l[1] != b'|' { continue; }
        let f = &l[2..];
        match l[0] {
            b'F' => { overview = field(f, 1) == b"1"; }
            b'G' => { NPTS = decode_points(field(f, 0), &mut RX, &mut RY); }
            b'K' => if NSTEPS < MAX_STEPS {
                let s = &mut STEPS[NSTEPS];
                s.at = num(field(f, 0)).clamp(0, 65535) as u16;
                s.code = field(f, 1).first().copied().unwrap_or(b'S');
                s.dist = num(field(f, 2)).clamp(0, 65535) as u16;
                s.exit = num(field(f, 3)).clamp(0, 12) as u8;
                s.tlen = copy_into(&mut s.text, field(f, 4));
                NSTEPS += 1;
            },
            b'D' => { DEST_LEN = copy_into(&mut DEST, field(f, 0)); TOTAL_M = num(field(f, 1)); }
            _ => {}
        }
    }
    if NPTS >= 2 && overview { OVERVIEW = true; OVERVIEW_MS = 8000; }
    else { OVERVIEW = false; OVERVIEW_MS = 0; S256 = zoom_target(); }
}
unsafe fn apply_streets() {
    NSPTS = 0; NSLINES = 0;
    for i in 0..NLINES {
        let l = ln(i);
        if l.len() < 2 || l[0] != b'L' || l[1] != b'|' { continue; }
        if NSLINES >= MAX_SLINES || NSPTS + 2 > MAX_SPTS { break; }
        let n = decode_points(&l[2..], &mut SX[NSPTS..], &mut SY[NSPTS..]);
        if n >= 2 { SSTART[NSLINES] = NSPTS as u16; NSLINES += 1; NSPTS += n; }
    }
    SSTART[NSLINES] = NSPTS as u16;
}
unsafe fn apply_places() {
    NPLACES = 0;
    for i in 0..NLINES {
        let l = ln(i);
        if l.len() < 2 || l[0] != b'Q' || l[1] != b'|' || NPLACES >= MAX_PLACES { continue; }
        let f = &l[2..];
        let p = &mut PLACES[NPLACES];
        p.nlen = copy_into(&mut p.name, field(f, 0));
        p.r10 = num(field(f, 1)).clamp(0, 50) as u8;
        p.reviews = num(field(f, 2)).clamp(0, 65535) as u16;
        p.ilen = copy_into(&mut p.info, field(f, 3));
        p.dist = num(field(f, 4)).clamp(0, 65535) as u16;
        p.open = num(field(f, 5)).clamp(0, 2) as u8;
        p.alen = copy_into(&mut p.addr, field(f, 6));
        NPLACES += 1;
    }
    CHOOSER = NPLACES > 0;
    PLACE_SEL = 0;
}
unsafe fn clear_all() {
    NPTS = 0; NSTEPS = 0; NSPTS = 0; NSLINES = 0; SSTART[0] = 0; NSTATE = 0; CHOOSER = false; NPLACES = 0;
    OVERVIEW = false; OVERVIEW_MS = 0; PAGE = 0; STEPS_TOP = 0; M_CODE = 0; M_DIST = -1; A_CODE = 0;
    REM_M = -1; REM_MIN = -1; ETA_LEN = 0; DEST_LEN = 0; STEP_IDX = 0; SEG_IDX = 0;
}

unsafe fn poll_local(ms: u32) {
    STATUS_MS = STATUS_MS.saturating_add(ms);
    if STATUS_MS < 1000 { return; }
    STATUS_MS = 0;
    let mut s = [0u8; 10];
    if device_status(s.as_mut_ptr(), 10) == 10 && s != STATUS { STATUS = s; DIRTY = true; }
}

// ---- Input ------------------------------------------------------------------
unsafe fn tap() {
    if CHOOSER {
        if PLACE_SEL < NPLACES {
            let mut b = Buffer::new(); b.add(b"{\"type\":\"go\",\"i\":"); b.num(PLACE_SEL); b.add(b"}");
            queue(b.bytes());
        } else {
            queue(b"{\"type\":\"close\"}");
        }
        CHOOSER = false;
        return;
    }
    if NSTATE == 3 { queue(b"{\"type\":\"done\"}"); return; }
    if NPTS < 2 || NSTATE == 6 { queue(b"{\"type\":\"sync\"}"); return; }
    if PAGE == 1 {
        STEPS_TOP += 6;
        let total = NSTEPS.saturating_sub((STEP_IDX + 1).min(NSTEPS));
        if STEPS_TOP >= total { STEPS_TOP = 0; }
        return;
    }
    if OVERVIEW { OVERVIEW = false; OVERVIEW_MS = 0; S256 = OV_S256; S256_T = zoom_target(); }
    else { OVERVIEW = true; OVERVIEW_MS = 0; }
}
unsafe fn swipe(next: bool) {
    if CHOOSER {
        let n = NPLACES + 1;
        PLACE_SEL = if next { (PLACE_SEL + 1) % n } else { (PLACE_SEL + n - 1) % n };
        return;
    }
    if NPTS < 2 || NSTATE == 3 || NSTATE == 6 { return; }
    PAGE ^= 1; STEPS_TOP = 0;
}

// ---- Exports ----------------------------------------------------------------
#[no_mangle] pub unsafe extern "C" fn app_init() {
    BRIDGE = companion::Companion::new(APP);
    clear_all();
    S256 = 512; S256_T = 512; DIRTY = true; STATUS_MS = 1000;
    queue(b"{\"type\":\"sync\"}");
    poll_local(0);
    render();
}

#[no_mangle] pub unsafe extern "C" fn app_render(ms: i32) {
    let ms = ms.clamp(0, 1000) as u32;
    if content::context_changed() { DIRTY = true; }
    if let Some(message) = BRIDGE.tick(ms) {
        match take(message) {
            b'n' => apply_nav(),
            b'r' => apply_route(),
            b's' => apply_streets(),
            b'p' => apply_places(),
            b'c' => clear_all(),
            _ => {}
        }
        DIRTY = true;
    }
    if OUT_LEN > 0 && BRIDGE.send(&OUT[..OUT_LEN]) { OUT_LEN = 0; }
    if OVERVIEW && OVERVIEW_MS > 0 {
        OVERVIEW_MS = OVERVIEW_MS.saturating_sub(ms);
        if OVERVIEW_MS == 0 { OVERVIEW = false; S256 = OV_S256; S256_T = zoom_target(); DIRTY = true; }
    }
    // Ease the follow-mode zoom towards the distance to the next turn.
    if !OVERVIEW && PAGE == 0 && NPTS >= 2 && !CHOOSER {
        S256_T = zoom_target();
        if S256 != S256_T {
            ANIM_MS = ANIM_MS.saturating_add(ms);
            if ANIM_MS >= 60 {
                let step = ((S256_T - S256) * ANIM_MS as i32 / 500).abs().max(1);
                ANIM_MS = 0;
                if (S256_T - S256).abs() <= step + 2 { S256 = S256_T; }
                else if S256_T > S256 { S256 += step; } else { S256 -= step; }
                DIRTY = true;
            }
        }
    }
    poll_local(ms);
    if DIRTY { DIRTY = false; render(); }
}

#[no_mangle] pub unsafe extern "C" fn app_event(e: i32) {
    match e {
        0 => tap(),
        1 => swipe(false),
        2 => swipe(true),
        _ => return,
    }
    render();
    DIRTY = false;
}

// The host releases this instance and its owned resources after this callback.
#[no_mangle] pub extern "C" fn app_stop() {}
