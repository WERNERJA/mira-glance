//! Shared glasses Listen meter: microphone energy, smoothed level and geometry.
#[link(wasm_import_module = "mira")]
extern "C" {
    fn fill_round_rect(x: i32, y: i32, w: i32, h: i32, r: i32, color: i32);
}

pub fn advance(level: f32, audio: u8, ms: u32) -> f32 {
    // Lift quieter speech while keeping headroom for louder peaks.
    // This only changes the drawing, never the microphone gain.
    let input = audio as f32 / 255.0;
    let target = 5.0 * input / (1.0 + 4.0 * input);
    let response = if target > level { 45.0 } else { 180.0 };
    let next = level + (target - level) * ms as f32 / (response + ms as f32);
    // Settle the animation only at zero input; never gate a nonzero level.
    if target == 0.0 && next < 0.015 { 0.0 } else { next }
}

pub unsafe fn draw_layer(x: i32, y: i32, level: f32, open: f32, grow: i32, ink: i32) {
    for (i, full) in [8u8, 14, 10, 12, 8].into_iter().enumerate() {
        let half = (((2.0 + (full as f32 - 2.0) * level) * open + 0.5) as i32).max(1);
        let bar_x = x + (i as i32 - 2) * 6;
        fill_round_rect(bar_x - 1 - grow, y - half - grow,
            2 + 2 * grow, 2 * half + 2 * grow, 1 + grow, ink);
    }
}
