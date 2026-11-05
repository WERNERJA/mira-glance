//! Glance: an at-a-glance dashboard for Mira glasses.
//! The phone companion fetches weather, calendar, news, markets and to-dos and
//! publishes one compact line-based snapshot; the glasses draw it locally and
//! add clock, date, battery and now-playing from local hardware.
#![no_std]
#![allow(static_mut_refs)]
#[path = "../Shared/companion.rs"] mod companion;
#[path = "../Shared/content.rs"] mod content;
#[path = "../Shared/replica_ui.rs"] mod ui;
use ui::*;

app_icon!(content::app_icon::Icon::new()
    .rect(2,3,12,11).rect(18,3,12,11).rect(2,18,28,11).line(6,23,26,23).0);

#[link(wasm_import_module = "mira")]
extern "C" { fn device_status(out: *mut u8, capacity: i32) -> i32; }
#[panic_handler] fn panic(_: &core::panic::PanicInfo) -> ! { loop {} }

const APP: &[u8] = b"Glance";
static mut BRIDGE: companion::Companion<4096, 64> = companion::Companion::new(APP);

// ---- Snapshot state ---------------------------------------------------------
const STATE_CAP: usize = 3600;
const MAX_LINES: usize = 64;
static mut STATE: [u8; STATE_CAP] = [0; STATE_CAP];
static mut STATE_LEN: usize = 0;
static mut LINES: [(u16, u16); MAX_LINES] = [(0, 0); MAX_LINES];
static mut NLINES: usize = 0;
static mut HAVE_DATA: bool = false;

// ---- UI state ---------------------------------------------------------------
const PAGE_CODES: &[u8] = b"oawnmt"; // overview, agenda, weather, news, markets, todo
static mut ORDER: [u8; 6] = *b"oawnmt";
static mut NPAGES: usize = 6;
static mut PAGE: usize = 0;
static mut DIRTY: bool = true;
static mut NEED_SEND: u8 = 1; // 1 = sync, 2 = refresh
static mut REFRESHING: u32 = 0; // ms remaining for the "refreshing" hint
static mut STATUS: [u8; 10] = [0; 10];
static mut STATUS_MS: u32 = 1000;
static mut MEDIA: [u8; 164] = [0; 164];
static mut MEDIA_LEN: i32 = 0;
static mut NEWS_TOP: usize = 0;

// ---- Small helpers ----------------------------------------------------------
fn field<'a>(l: &'a [u8], index: usize) -> &'a [u8] {
    l.split(|b| *b == b'|').nth(index).unwrap_or(&[])
}
unsafe fn ln(i: usize) -> &'static [u8] {
    let (s, e) = LINES[i];
    &STATE[s as usize..e as usize]
}
/// Calls `f` for every snapshot line with the given tag, up to `max`.
unsafe fn each(tag: u8, max: usize, mut f: impl FnMut(usize, &'static [u8]) -> bool) -> usize {
    let mut n = 0;
    for i in 0..NLINES {
        if n >= max { break; }
        let l = ln(i);
        if l.len() >= 2 && l[0] == tag && l[1] == b'|' {
            if !f(n, &l[2..]) { break; }
            n += 1;
        }
    }
    n
}
unsafe fn first(tag: u8) -> &'static [u8] {
    for i in 0..NLINES {
        let l = ln(i);
        if l.len() >= 2 && l[0] == tag && l[1] == b'|' { return &l[2..]; }
    }
    &[]
}
fn two(out: &mut Buffer, v: u8) { out.add(&[b'0' + v / 10 % 10, b'0' + v % 10]); }

/// Draws one ellipsized line. `font` is a pixel size: 18, 24, 32 or 40.
unsafe fn t(x: i32, y: i32, w: i32, font: i32, gray: i32, align: i32, v: &[u8]) {
    let h = match font { 18 => 24, 24 => 33, 32 => 47, _ => 57 };
    single(x, y, w, h, gray, font, align, v);
}
/// Wrapped text, up to `max` lines.
unsafe fn wrap(x: i32, y: i32, w: i32, font: i32, gray: i32, spacing: i32, max: usize, v: &[u8]) {
    label(x, y, w, spacing * max as i32, gray, font, 0, spacing, max, true, false, v);
}
fn bwidth(v: &[u8], font: i32) -> i32 { width(core::str::from_utf8(v).unwrap_or(""), font) }

/// Temperature with a drawn degree ring (fonts do not reliably carry the glyph).
unsafe fn temp(x: i32, y: i32, font: i32, gray: i32, align: i32, v: &[u8]) -> i32 {
    if v.is_empty() { return 0; }
    let w = bwidth(v, font);
    let r = (font / 9).max(2);
    let total = w + r * 2 + 4;
    let left = match align { 1 => x - total / 2, 2 => x - total, _ => x };
    t(left, y, w + 8, font, gray, 0, v);
    stroke_circle(left + w + r + 2, y + font / 5 + r, r, if font >= 32 { 2 } else { 1 }, gray);
    total
}

// ---- Weather icons ----------------------------------------------------------
unsafe fn cloud(cx: i32, cy: i32, s: i32, gray: i32) {
    fill_circle(cx - s * 9 / 32, cy + s * 2 / 32, s * 7 / 32, gray);
    fill_circle(cx + s * 1 / 32, cy - s * 4 / 32, s * 10 / 32, gray);
    fill_circle(cx + s * 10 / 32, cy + s * 3 / 32, s * 6 / 32, gray);
    fill_round_rect(cx - s * 9 / 32, cy + s * 2 / 32, s * 19 / 32, s * 7 / 32, s * 3 / 32, gray);
}
unsafe fn sun(cx: i32, cy: i32, s: i32, gray: i32) {
    let r = s * 7 / 32;
    fill_circle(cx, cy, r, gray);
    const D: [(i32, i32); 8] = [(16, 0), (11, 11), (0, 16), (-11, 11), (-16, 0), (-11, -11), (0, -16), (11, -11)];
    let w = (s / 16).max(2);
    for (dx, dy) in D {
        line(cx + dx * s * 11 / 512, cy + dy * s * 11 / 512, cx + dx * s * 15 / 512, cy + dy * s * 15 / 512, w, gray);
    }
}
unsafe fn moon(cx: i32, cy: i32, s: i32, gray: i32) {
    fill_circle(cx, cy, s * 9 / 32, gray);
    fill_circle(cx + s * 5 / 32, cy - s * 4 / 32, s * 8 / 32, 0);
}
/// Icon codes: s sun, m moon, p partly cloudy, q partly cloudy night,
/// c cloud, r rain, d drizzle, t thunder, n snow, f fog.
unsafe fn wx(cx: i32, cy: i32, s: i32, code: u8) {
    let w = (s / 16).max(2);
    match code {
        b's' => sun(cx, cy, s, 15),
        b'm' => moon(cx, cy, s, 14),
        b'p' | b'q' => {
            if code == b'p' { sun(cx - s * 6 / 32, cy - s * 6 / 32, s * 3 / 4, 15); }
            else { moon(cx - s * 6 / 32, cy - s * 6 / 32, s * 3 / 4, 14); }
            cloud(cx + s * 3 / 32, cy + s * 4 / 32, s * 3 / 4, 0);
            cloud(cx + s * 2 / 32, cy + s * 3 / 32, s * 3 / 4 - 4, 11);
        }
        b'c' => cloud(cx, cy, s, 11),
        b'r' | b'd' | b't' | b'n' => {
            cloud(cx, cy - s * 5 / 32, s, 11);
            let base = cy + s * 7 / 32;
            for i in 0..3 {
                let x = cx - s * 8 / 32 + i * s * 8 / 32;
                match code {
                    b'n' => fill_circle(x, base + s * 4 / 32 + (i % 2) * s * 3 / 32, (s / 18).max(2), 15),
                    b'd' => fill_circle(x, base + s * 5 / 32, (s / 22).max(2), 13),
                    b't' => {}
                    _ => line(x + s * 2 / 32, base + s * 1 / 32, x - s * 2 / 32, base + s * 9 / 32, w, 13),
                }
            }
            if code == b't' {
                fill_triangle(cx + s * 2 / 32, base, cx - s * 6 / 32, base + s * 7 / 32, cx, base + s * 7 / 32, 15);
                fill_triangle(cx - s * 1 / 32, base + s * 5 / 32, cx + s * 5 / 32, base + s * 5 / 32, cx - s * 4 / 32, base + s * 14 / 32, 15);
            }
        }
        b'f' => {
            for i in 0..4 {
                let y = cy - s * 9 / 32 + i * s * 6 / 32;
                let inset = if i % 2 == 0 { 0 } else { s * 4 / 32 };
                line(cx - s * 12 / 32 + inset, y, cx + s * 12 / 32 - inset / 2, y, w, 10);
            }
        }
        _ => stroke_circle(cx, cy, s / 4, w, 8),
    }
}

// ---- Chrome-like pieces ------------------------------------------------------
const DAYS: [&[u8]; 7] = [b"zondag", b"maandag", b"dinsdag", b"woensdag", b"donderdag", b"vrijdag", b"zaterdag"];
const MONTHS: [&[u8]; 12] = [b"januari", b"februari", b"maart", b"april", b"mei", b"juni", b"juli",
    b"augustus", b"september", b"oktober", b"november", b"december"];

unsafe fn battery(x: i32, y: i32) {
    let pct = STATUS[0];
    if pct > 100 { return; }
    let mut b = Buffer::new(); b.num(pct as usize); b.add(b"%");
    t(x - 120, y - 2, 70, 18, 10, 2, b.bytes());
    stroke_round_rect(x - 44, y, 36, 18, 4, 2, 10);
    fill_rect(x - 8, y + 5, 3, 8, 10);
    let fill = (pct as i32 * 28 / 100).max(2);
    fill_rect(x - 40, y + 4, fill, 10, if pct <= 15 { 15 } else { 11 });
    if STATUS[1] != 0 { // charging bolt
        fill_triangle(x - 24, y + 2, x - 32, y + 10, x - 25, y + 10, 0);
        fill_triangle(x - 27, y + 8, x - 20, y + 8, x - 28, y + 16, 0);
    }
}

unsafe fn header(title: &[u8]) {
    t(28, 20, 360, 32, 15, 0, title);
    let mut b = Buffer::new();
    if STATUS[4] != 0 { two(&mut b, STATUS[5]); b.add(b":"); two(&mut b, STATUS[6]); }
    t(330, 26, 150, 24, 10, 2, b.bytes());
    battery(620, 30);
    line(28, 72, 612, 72, 1, 5);
}

unsafe fn footer() {
    // Page dots.
    let gap = 18; let n = NPAGES as i32;
    let x0 = 320 - (n - 1) * gap / 2;
    for i in 0..n {
        if i as usize == PAGE { fill_circle(x0 + i * gap, 458, 5, 15); }
        else { stroke_circle(x0 + i * gap, 458, 4, 1, 7); }
    }
    let hint: &[u8] = if STATUS[3] == 0 && STATUS[0] <= 100 { b"Telefoon niet verbonden" }
        else if REFRESHING > 0 { b"Vernieuwen..." }
        else if !HAVE_DATA { b"Wacht op telefoon..." }
        else { b"" };
    if !hint.is_empty() { t(28, 446, 240, 18, 9, 0, hint); }
    let upd = first(b'U');
    if !upd.is_empty() && hint.is_empty() {
        let mut b = Buffer::new(); b.add(b"bijgewerkt "); b.add(upd);
        t(400, 446, 212, 18, 7, 2, b.bytes());
    }
}

// ---- Pages ------------------------------------------------------------------
unsafe fn event_row(x: i32, y: i32, w: i32, ev: &[u8], show_day: bool) {
    let when = field(ev, 0); let title = field(ev, 1); let place = field(ev, 2); let day = field(ev, 3);
    t(x, y, 104, 24, 15, 0, when);
    t(x + 110, y, w - 110, 24, 15, 0, title);
    let mut b = Buffer::new();
    if show_day && !day.is_empty() && day != b"Vandaag" { b.add(day); if !place.is_empty() { b.add(b"  -  "); } }
    b.add(place);
    if b.n > 0 { t(x + 110, y + 30, w - 110, 18, 9, 0, b.bytes()); }
}

unsafe fn overview() {
    // Clock and date.
    let mut b = Buffer::new();
    if STATUS[4] != 0 { two(&mut b, STATUS[5]); b.add(b":"); two(&mut b, STATUS[6]); } else { b.add(b"--:--"); }
    t(28, 14, 160, 40, 15, 0, b.bytes());
    let cw = bwidth(b.bytes(), 40);
    if STATUS[4] != 0 && STATUS[9] < 7 && STATUS[7] >= 1 && STATUS[7] <= 12 {
        t(28 + cw + 22, 16, 260, 18, 10, 0, DAYS[STATUS[9] as usize]);
        let mut d = Buffer::new(); d.num(STATUS[8] as usize); d.add(b" "); d.add(MONTHS[STATUS[7] as usize - 1]);
        t(28 + cw + 22, 38, 260, 24, 15, 0, d.bytes());
    }
    battery(620, 30);
    line(28, 78, 612, 78, 1, 5);

    // Weather (right column).
    let w = first(b'W');
    if !w.is_empty() {
        let code = field(w, 5).first().copied().unwrap_or(b'c');
        wx(400, 136, 72, code);
        temp(446, 104, 40, 15, 0, field(w, 0));
        t(446, 154, 166, 18, 11, 0, field(w, 1));
        let mut x = 360;
        t(x, 190, 44, 18, 8, 0, b"max"); x += 40;
        x += temp(x, 190, 18, 13, 0, field(w, 2)) + 14;
        t(x, 190, 40, 18, 8, 0, b"min"); x += 36;
        temp(x, 190, 18, 13, 0, field(w, 3));
        t(360, 214, 252, 18, 7, 0, field(w, 4));
    } else {
        t(360, 120, 252, 18, 8, 0, b"Geen weerdata");
    }

    // Agenda (left column).
    t(28, 94, 300, 18, 8, 0, b"AGENDA");
    let n = each(b'E', 3, |i, ev| { event_row(28, 122 + i as i32 * 66, 312, ev, true); true });
    if n == 0 { t(28, 124, 300, 24, 9, 0, b"Niets gepland"); }
    line(344, 96, 344, 300, 1, 4);

    // Bottom strip: now playing, else the top headline, else the first to-do.
    line(28, 318, 612, 318, 1, 5);
    let playing = MEDIA_LEN > 2 && MEDIA[0] != 0;
    if playing {
        let m = &MEDIA[2..MEDIA_LEN as usize];
        let title = m.split(|c| *c == 0).next().unwrap_or(&[]);
        let artist = m.split(|c| *c == 0).nth(1).unwrap_or(&[]);
        // Note glyph.
        fill_circle(40, 384, 7, 15); line(46, 384, 46, 360, 2, 15); line(46, 360, 56, 367, 2, 15);
        t(28, 334, 300, 18, 8, 0, if MEDIA[1] != 0 { b"NU AAN HET SPELEN" } else { b"GEPAUZEERD" });
        t(72, 358, 540, 24, 15, 0, title);
        t(72, 390, 540, 18, 10, 0, artist);
    } else {
        let news = first(b'N');
        if !news.is_empty() {
            t(28, 334, 584, 18, 8, 0, field(news, 0));
            wrap(28, 358, 584, 24, 15, 30, 2, field(news, 1));
        } else {
            let todo = first(b'T');
            if !todo.is_empty() {
                t(28, 334, 584, 18, 8, 0, b"TO-DO");
                stroke_rect(28, 364, 18, 18, 2, 12);
                t(58, 358, 554, 24, 15, 0, field(todo, 0));
            }
        }
    }
}

unsafe fn agenda() {
    header(b"Agenda");
    let mut y = 86; let mut last: &[u8] = &[];
    let n = each(b'E', 8, |_, ev| {
        let day = field(ev, 3);
        let need_header = day != last;
        let needed = if need_header { 28 } else { 0 } + 58;
        if y + needed > 440 { return false; }
        if need_header {
            t(28, y, 584, 18, 8, 0, day); y += 28; last = day;
        }
        event_row(28, y, 584, ev, false);
        y += 58;
        true
    });
    if n == 0 {
        t(28, 120, 584, 24, 10, 0, b"Geen afspraken gepland");
        t(28, 156, 584, 18, 7, 0, b"Voeg een agenda-link toe in de telefoon-app.");
    }
}

unsafe fn weather() {
    header(b"Weer");
    let w = first(b'W');
    if w.is_empty() {
        t(28, 120, 584, 24, 10, 0, b"Geen weerdata");
        t(28, 156, 584, 18, 7, 0, b"Kies een locatie in de telefoon-app.");
        return;
    }
    wx(84, 136, 88, field(w, 5).first().copied().unwrap_or(b'c'));
    temp(150, 92, 40, 15, 0, field(w, 0));
    t(150, 144, 250, 24, 12, 0, field(w, 1));
    t(150, 176, 250, 18, 8, 0, field(w, 4));
    let extra: [(&[u8], &[u8], &[u8]); 3] = [
        (b"Gevoel", field(w, 6), b""), (b"Wind", field(w, 7), b" km/u"), (b"Neerslag", field(w, 8), b"%")];
    for (i, (k, v, unit)) in extra.iter().enumerate() {
        let y = 96 + i as i32 * 34;
        t(420, y, 90, 18, 8, 0, k);
        if i == 0 { temp(612, y - 2, 24, 13, 2, v); }
        else { let mut b = Buffer::new(); b.add(v); b.add(unit); t(500, y - 2, 112, 24, 13, 2, b.bytes()); }
    }
    // Hourly strip.
    line(28, 208, 612, 208, 1, 5);
    each(b'H', 6, |i, h| {
        let cx = 28 + 49 + i as i32 * 97;
        t(cx - 48, 218, 96, 18, 9, 1, field(h, 0));
        wx(cx, 268, 36, field(h, 2).first().copied().unwrap_or(b'c'));
        temp(cx, 292, 24, 15, 1, field(h, 1));
        let rain = field(h, 3);
        if !rain.is_empty() && rain != b"0" {
            let mut b = Buffer::new(); b.add(rain); b.add(b"%"); t(cx - 48, 322, 96, 18, 8, 1, b.bytes());
        }
        true
    });
    // Daily rows.
    line(28, 350, 612, 350, 1, 5);
    each(b'D', 3, |i, dd| {
        let y = 358 + i as i32 * 28;
        t(28, y, 160, 18, 11, 0, field(dd, 0));
        wx(230, y + 10, 24, field(dd, 3).first().copied().unwrap_or(b'c'));
        let mut x = 270;
        x += temp(x, y, 18, 15, 0, field(dd, 1)) + 6;
        t(x, y, 20, 18, 7, 0, b"/"); x += 16;
        temp(x, y, 18, 9, 0, field(dd, 2));
        let r = field(dd, 4);
        if !r.is_empty() && r != b"0" {
            let mut b = Buffer::new(); b.add(r); b.add(b"% kans op neerslag");
            t(380, y, 232, 18, 8, 2, b.bytes());
        }
        true
    });
}

unsafe fn news() {
    header(b"Nieuws");
    let total = each(b'N', 12, |_, _| true);
    if total == 0 {
        t(28, 120, 584, 24, 10, 0, b"Geen nieuws");
        t(28, 156, 584, 18, 7, 0, b"Kies een nieuwsbron in de telefoon-app.");
        return;
    }
    if NEWS_TOP >= total { NEWS_TOP = 0; }
    let start = NEWS_TOP;
    let mut y = 86;
    let mut shown = 0;
    each(b'N', 12, |i, item| {
        if i < start { return true; }
        if y + 82 > 418 { return false; }
        fill_circle(34, y + 44, 3, 10);
        t(48, y + 4, 564, 18, 8, 0, field(item, 0));
        wrap(48, y + 28, 564, 24, 15, 28, 2, field(item, 1));
        y += 82; shown += 1;
        true
    });
    if total > 4 {
        let mut b = Buffer::new(); b.num(start + 1); b.add(b"-"); b.num((start + shown).min(total)); b.add(b" van "); b.num(total);
        b.add(b", tik voor meer");
        t(28, 422, 584, 18, 7, 2, b.bytes());
    }
}

unsafe fn markets() {
    header(b"Markten");
    let n = each(b'S', 7, |i, s| {
        let y = 88 + i as i32 * 50;
        let change = field(s, 2);
        let up = !change.starts_with(b"-");
        t(28, y, 240, 24, 15, 0, field(s, 0));
        t(280, y, 180, 24, 13, 2, field(s, 1));
        if !change.is_empty() {
            let tx = 496; let ty = y + 15;
            if up { fill_triangle(tx, ty - 8, tx - 9, ty + 7, tx + 9, ty + 7, 15); }
            else { fill_triangle(tx - 9, ty - 7, tx + 9, ty - 7, tx, ty + 8, 8); }
            let mut b = Buffer::new(); b.add(change); b.add(b"%");
            t(512, y, 100, 24, if up { 15 } else { 9 }, 2, b.bytes());
        }
        if i > 0 { line(28, y - 9, 612, y - 9, 1, 3); }
        true
    });
    if n == 0 {
        t(28, 120, 584, 24, 10, 0, b"Geen koersen");
        t(28, 156, 584, 18, 7, 0, b"Voeg symbolen toe in de telefoon-app.");
    }
}

unsafe fn todos() {
    header(b"To-do");
    let n = each(b'T', 8, |i, item| {
        let y = 90 + i as i32 * 44;
        let done = field(item, 1) == b"1";
        stroke_round_rect(30, y + 4, 22, 22, 4, 2, if done { 7 } else { 13 });
        if done { line(34, y + 15, 40, y + 21, 2, 9); line(40, y + 21, 49, y + 8, 2, 9); }
        t(66, y, 546, 24, if done { 7 } else { 15 }, 0, field(item, 0));
        true
    });
    if n == 0 { t(28, 120, 584, 24, 10, 0, b"Alles gedaan!"); }
}

unsafe fn render() {
    begin();
    match ORDER[PAGE.min(NPAGES - 1)] {
        b'a' => agenda(),
        b'w' => weather(),
        b'n' => news(),
        b'm' => markets(),
        b't' => todos(),
        _ => overview(),
    }
    footer();
    content::cursor(612, 458, false);
    present();
}

// ---- Snapshot intake --------------------------------------------------------
fn hex(b: u8) -> Option<u32> {
    match b { b'0'..=b'9' => Some((b - b'0') as u32), b'a'..=b'f' => Some((b - b'a' + 10) as u32),
        b'A'..=b'F' => Some((b - b'A' + 10) as u32), _ => None }
}
fn put_utf8(out: &mut [u8], n: &mut usize, cp: u32) {
    let mut tmp = [0u8; 4];
    let s = char::from_u32(cp).unwrap_or('?').encode_utf8(&mut tmp);
    for b in s.bytes() { if *n < out.len() { out[*n] = b; *n += 1; } }
}
/// Copies the JSON string value of key `"d"` into STATE, unescaping it.
unsafe fn take(json: &[u8]) -> bool {
    let key = b"\"d\":\"";
    let mut start = None;
    if json.len() >= key.len() {
        for i in 0..=json.len() - key.len() { if &json[i..i + key.len()] == key { start = Some(i + key.len()); break; } }
    }
    let Some(mut i) = start else { return false };
    let mut n = 0;
    while i < json.len() && n < STATE_CAP {
        let b = json[i];
        if b == b'"' { break; }
        if b == b'\\' && i + 1 < json.len() {
            let e = json[i + 1]; i += 2;
            match e {
                b'n' => { STATE[n] = b'\n'; n += 1; }
                b't' | b'r' | b'b' | b'f' => { STATE[n] = b' '; n += 1; }
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
                    put_utf8(&mut STATE, &mut n, cp);
                }
                _ => { STATE[n] = e; n += 1; }
            }
            continue;
        }
        STATE[n] = b; n += 1; i += 1;
    }
    STATE_LEN = n;
    // Index lines.
    NLINES = 0;
    let mut s = 0;
    for j in 0..=STATE_LEN {
        if j == STATE_LEN || STATE[j] == b'\n' {
            if j > s && NLINES < MAX_LINES { LINES[NLINES] = (s as u16, j as u16); NLINES += 1; }
            s = j + 1;
        }
    }
    // Page order from the phone ("P|oawnmt"); unknown codes are ignored.
    let p = first(b'P');
    if !p.is_empty() {
        let mut k = 0;
        ORDER[0] = b'o'; k += 1;
        for &c in p {
            if k >= ORDER.len() { break; }
            if c != b'o' && PAGE_CODES.contains(&c) && !ORDER[..k].contains(&c) { ORDER[k] = c; k += 1; }
        }
        NPAGES = k;
        if PAGE >= NPAGES { PAGE = 0; }
    }
    HAVE_DATA = true;
    true
}

unsafe fn poll_local(ms: u32) {
    STATUS_MS = STATUS_MS.saturating_add(ms);
    if STATUS_MS < 1000 { return; }
    STATUS_MS = 0;
    let mut s = [0u8; 10];
    if device_status(s.as_mut_ptr(), 10) == 10 && s != STATUS { STATUS = s; DIRTY = true; }
    let mut m = [0u8; 164];
    let n = content::media::read(&mut m);
    let n = if n < 0 { 0 } else { n };
    if n != MEDIA_LEN || m[..n as usize] != MEDIA[..n as usize] {
        MEDIA = m; MEDIA_LEN = n;
        if ORDER[PAGE] == b'o' { DIRTY = true; }
    }
}

// ---- Exports ----------------------------------------------------------------
#[no_mangle] pub unsafe extern "C" fn app_init() {
    BRIDGE = companion::Companion::new(APP);
    PAGE = 0; NEED_SEND = 1; DIRTY = true; STATUS_MS = 1000; REFRESHING = 0; NEWS_TOP = 0;
    poll_local(0);
    render();
}

#[no_mangle] pub unsafe extern "C" fn app_render(ms: i32) {
    let ms = ms.clamp(0, 1000) as u32;
    if content::context_changed() { DIRTY = true; }
    if let Some(message) = BRIDGE.tick(ms) {
        if take(message) { REFRESHING = 0; DIRTY = true; }
    }
    if NEED_SEND != 0 {
        let msg: &[u8] = if NEED_SEND == 2 { b"{\"type\":\"refresh\"}" } else { b"{\"type\":\"sync\"}" };
        if BRIDGE.send(msg) { NEED_SEND = 0; }
    }
    if REFRESHING > 0 {
        REFRESHING = REFRESHING.saturating_sub(ms);
        if REFRESHING == 0 { DIRTY = true; }
    }
    poll_local(ms);
    if DIRTY { DIRTY = false; render(); }
}

#[no_mangle] pub unsafe extern "C" fn app_event(e: i32) {
    match e {
        1 => { PAGE = if PAGE == 0 { NPAGES - 1 } else { PAGE - 1 }; NEWS_TOP = 0; }
        2 => { PAGE = (PAGE + 1) % NPAGES; NEWS_TOP = 0; }
        0 => {
            if ORDER[PAGE] == b'n' && each(b'N', 12, |_, _| true) > 4 {
                NEWS_TOP += 4;
            } else {
                NEED_SEND = 2; REFRESHING = 8000;
            }
        }
        _ => return,
    }
    DIRTY = true;
    render();
    DIRTY = false;
}

// The host releases this instance and its owned resources after this callback.
#[no_mangle] pub extern "C" fn app_stop() {}
