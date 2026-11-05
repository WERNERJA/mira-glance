//! App-owned, 32 × 32 monochrome sidebar icons. No runtime code is executed
//! when the launcher reads the `mira.icon` custom section from an installed app.
#![allow(dead_code)]

#[macro_export]
macro_rules! app_icon {
    ($pixels:expr) => {
        #[used]
        #[link_section = "mira.icon"]
        static APP_ICON: [u8; 128] = $pixels;
    };
}

/// Small const drawing helper; authors can also pass their own MSB-first mask.
pub struct Icon(pub [u8; 128]);
impl Icon {
    pub const fn new() -> Self { Self([0; 128]) }
    const fn pixel(mut self, x: i32, y: i32) -> Self {
        if x >= 0 && x < 32 && y >= 0 && y < 32 {
            self.0[(y * 4 + x / 8) as usize] |= 0x80 >> (x % 8);
        }
        self
    }
    /// Two-pixel stroke, with coordinates in the 32 px icon cell.
    pub const fn line(mut self, mut x: i32, mut y: i32, x1: i32, y1: i32) -> Self {
        let dx = (x1 - x).abs(); let dy = -(y1 - y).abs();
        let sx = if x < x1 { 1 } else { -1 }; let sy = if y < y1 { 1 } else { -1 };
        let mut error = dx + dy;
        loop {
            self = self.pixel(x, y).pixel(x + 1, y).pixel(x, y + 1);
            if x == x1 && y == y1 { break; }
            let twice = error * 2;
            if twice >= dy { error += dy; x += sx; }
            if twice <= dx { error += dx; y += sy; }
        }
        self
    }
    pub const fn rect(self, x: i32, y: i32, w: i32, h: i32) -> Self {
        self.line(x,y,x+w,y).line(x+w,y,x+w,y+h)
            .line(x+w,y+h,x,y+h).line(x,y+h,x,y)
    }
    pub const fn circle(mut self, cx: i32, cy: i32, r: i32) -> Self {
        let mut y = -r;
        while y <= r {
            let mut x = -r;
            while x <= r {
                let d = x*x + y*y;
                if d <= r*r && d >= (r-2)*(r-2) { self = self.pixel(cx+x,cy+y); }
                x += 1;
            }
            y += 1;
        }
        self
    }
}
