//! Drawing helpers for head-controlled SDK games.
use super::head_game::{Control, Phase, Pose};
#[link(wasm_import_module = "mira")]
extern "C" {
    pub fn begin(); pub fn present();
    pub fn line(x0:i32,y0:i32,x1:i32,y1:i32,width:i32,gray:i32);
    pub fn fill_rect(x:i32,y:i32,w:i32,h:i32,gray:i32);
    pub fn stroke_rect(x:i32,y:i32,w:i32,h:i32,width:i32,gray:i32);
    pub fn fill_circle(x:i32,y:i32,r:i32,gray:i32);
    pub fn stroke_circle(x:i32,y:i32,r:i32,width:i32,gray:i32);
    pub fn fill_triangle(x0:i32,y0:i32,x1:i32,y1:i32,x2:i32,y2:i32,gray:i32);
    fn text(x:i32,y:i32,w:i32,h:i32,gray:i32,font:i32,align:i32,spacing:i32,p:*const u8,n:i32);
    fn imu_yaw_cdeg()->i32; fn imu_pitch_cdeg()->i32; fn imu_ready()->i32;
}
pub unsafe fn pose() -> Pose {
    // Getters claim sampling even before the first calibrated sample exists.
    let yaw = imu_yaw_cdeg(); let pitch = imu_pitch_cdeg();
    Pose { horizontal: yaw, pitch, ready: imu_ready() != 0 }
}
pub unsafe fn label(x:i32,y:i32,w:i32,gray:i32,font:i32,align:i32,s:&[u8]) {
    text(x,y,w,32,gray,font,align,28,s.as_ptr(),s.len() as i32);
}
pub unsafe fn center(y:i32,s:&[u8]) { label(82,y,476,15,1,1,s); }
pub unsafe fn number(x:i32,y:i32,w:i32,n:u32) {
    let mut bytes=[0;10]; let mut pos=10; let mut v=n;
    loop { pos-=1; bytes[pos]=b'0'+(v%10) as u8; v/=10; if v==0 { break; } }
    label(x,y,w,15,1,0,&bytes[pos..]);
}
pub unsafe fn meter(x:i32,y:i32,w:i32,value:i32,max:i32) {
    fill_rect(x,y,w,4,4);
    let filled=value.clamp(0,max)*w/max;
    if filled>0 { fill_rect(x,y,filled,4,15); }
}
pub unsafe fn card(title:&[u8], subtitle:&[u8]) {
    fill_rect(92,173,456,134,0);
    label(110,188,420,15,0,1,title);
    center(238,subtitle);
}
pub unsafe fn overlay(c:&Control,title:&[u8],_hint:&[u8]) {
    match c.phase {
        Phase::Waiting => card(title,b"Waiting for head tracking"),
        Phase::Paused => card(b"PAUSED",if c.head_pause {b"Lift your head to resume"} else {b"Tap to resume"}),
        Phase::Live => {}
    }
}
