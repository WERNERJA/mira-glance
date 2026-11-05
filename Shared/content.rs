//! Services for the active content module. The resident owns all global UI.
#![allow(dead_code)]
#[path="app_icon.rs"] pub mod app_icon;
#[path="casa_data.rs"] pub mod data;
pub mod push_data;
pub mod media;
pub mod imu;
#[link(wasm_import_module="mira")]
extern "C" {
    fn gesture_policy(gesture:i32,local:i32)->i32;
    fn content_cursor(x:i32,y:i32,flags:i32);
    fn content_info(field:i32)->i32;
    fn app_launch(id:*const u8,len:i32)->i32;
}
static mut LAST_CONTEXT:i32=-1;
/// Sets the cursor anchor until replaced or app switch.
/// Visibility controls the content pointer; resident controls keep their own.
pub unsafe fn cursor(x:i32,y:i32,visible:bool) { content_cursor(x,y,visible as i32); }
/// Claims double tap for this active instance. Switching apps revokes the claim.
pub unsafe fn double_tap_local(local:bool) { gesture_policy(3,local as i32); }
/// Claims long-click and hold edges for this instance; app switches revoke them.
/// Check chrome visibility before starting a hold, but always handle its release.
pub unsafe fn holds_local(local:bool) {
    for event in [9,10,12] { gesture_policy(event,local as i32); }
}
/// Returns the resident's sidebar/overlay occlusion hints for pacing local UI.
pub unsafe fn sidebar_open()->bool { content_info(5)&1!=0 }
pub unsafe fn overlay_visible()->bool { content_info(5)&2!=0 }
pub unsafe fn context_changed()->bool {
    let current=content_info(5); let changed=current!=LAST_CONTEXT; LAST_CONTEXT=current; changed
}
pub unsafe fn launch_app(id:&[u8]) { app_launch(id.as_ptr(),id.len() as i32); }
