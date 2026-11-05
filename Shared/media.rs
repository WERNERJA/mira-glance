//! Local media primitives. The Bluetooth chip owns AVRCP; the SDK owns the UI.
#[link(wasm_import_module = "mira")]
extern "C" {
    fn media_state(out: *mut u8, capacity: i32) -> i32;
    fn media_command(command: i32) -> i32;
}

/// Available, playing, then NUL-delimited title and artist. No phone RPC.
pub unsafe fn read(out: &mut [u8]) -> i32 {
    let count = media_state(out.as_mut_ptr(), out.len() as i32);
    if count > out.len() as i32 { return -1; }
    // Keep song metadata on one line; rendering/truncation stays in the SDK.
    if count > 2 {
        for byte in &mut out[2..count as usize] {
            if *byte == b'\n' || *byte == b'\r' || *byte == b'\t' { *byte = b' '; }
        }
    }
    count
}

/// Previous, play/pause, next, volume up, volume down (0..4).
/// True means accepted by the glasses transport, not confirmed by the player.
pub unsafe fn command(command: u8) -> bool { media_command(command as i32) == 1 }
