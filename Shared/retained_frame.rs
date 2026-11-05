//! Retain app drawing while an independent SDK overlay changes.
//! Call begin(content_changed), draw app content and checkpoint only when true,
//! then draw overlays and present as usual. No duplicate WASM drawing buffer.
#[link(wasm_import_module = "mira")]
extern "C" {
    fn begin();
    fn frame_checkpoint() -> i32;
    fn frame_restore() -> i32;
}
pub unsafe fn start(content_changed: bool) -> bool {
    if !content_changed && frame_restore() != 0 { return false; }
    begin();
    true
}
pub unsafe fn checkpoint() { frame_checkpoint(); }
