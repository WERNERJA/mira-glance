//! Casa's cursor-adjacent text bubble, shared with apps that draw their own cursor.
#[path = "replica_ui.rs"]
mod typography;

#[link(wasm_import_module = "mira")]
extern "C" {
    fn fill_round_rect(x: i32, y: i32, w: i32, h: i32, radius: i32, ink: i32);
    fn stroke_round_rect(x: i32, y: i32, w: i32, h: i32, radius: i32, border: i32, ink: i32);
    fn text(x: i32, y: i32, w: i32, h: i32, ink: i32, font: i32, align: i32,
            spacing: i32, value: *const u8, length: i32);
}

// Width is a maximum; short text hugs its longest line with 11 px side insets.
// Preserve the resident's bubble geometry, screen clamping and 24 px typography.
pub unsafe fn draw(cursor_x: i32, cursor_y: i32, width: i32, border_width: i32, value: &[u8]) {
    draw_with_clearance(cursor_x, cursor_y, width, border_width, value, 0);
}

// Larger cursor shapes reuse the same bubble, leaving room for their contour.
pub unsafe fn draw_with_clearance(cursor_x: i32, cursor_y: i32, width: i32, border_width: i32, value: &[u8], clearance: i32) {
    if value.is_empty() { return; }
    let width = core::str::from_utf8(value).map(|value| {
        let text_width = value.split('\n').take(3)
            .map(|line| typography::width(line, 24)).max().unwrap_or(0);
        (text_width + 22).min(width)
    }).unwrap_or(width);
    draw_fixed_with_clearance(cursor_x, cursor_y, width, border_width, value, clearance);
}

// Animated labels can reserve their full width while the visible text changes.
pub unsafe fn draw_fixed_with_clearance(cursor_x: i32, cursor_y: i32, width: i32, border_width: i32, value: &[u8], clearance: i32) {
    if value.is_empty() { return; }
    let lines = (value.iter().filter(|byte| **byte == b'\n').count() as i32 + 1).min(3);
    let height = 47 + (lines - 1) * 27;
    let x = (if cursor_x >= 320 { cursor_x - width - 18 - clearance } else { cursor_x + 24 + clearance })
        .clamp(8, 640 - width - 8);
    let y = (cursor_y + 10 - height / 2).clamp(42, 480 - height - 2);
    fill_round_rect(x, y, width, height, 10, 0);
    stroke_round_rect(x, y, width, height, 10, border_width, 15);
    text(x + 11, y + 7, width - 22, height - 14, 15, 0, 0, 27, value.as_ptr(), value.len() as i32);
}
