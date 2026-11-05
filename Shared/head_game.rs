//! Automatic IMU setup and interruption handling for head-controlled games.
pub const STEP: i32 = 20;
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Phase { Waiting, Live, Paused }
#[derive(Clone, Copy)]
pub struct Pose { pub horizontal: i32, pub pitch: i32, pub ready: bool }
pub fn delta(angle: i32, center: i32) -> i32 {
    (angle - center + 54_000) % 36_000 - 18_000
}
pub struct Control {
    pub phase: Phase, pub x: i32, pub y: i32,
    pub center_horizontal: i32, pub center_pitch: i32,
    pub head_pause: bool,
    initialized: bool, blocked: bool, down: i32, pause_pitch: i32,
}
impl Control {
    pub const fn new() -> Self {
        Self { phase: Phase::Waiting, x: 0, y: 0, center_horizontal: 0,
            center_pitch: 0, head_pause: true, initialized: false, blocked: false, down: 0, pause_pitch: 0 }
    }
    // Games can reserve the full head-motion range for play. Only deliberate
    // taps may pause/resume; looking down must not freeze the simulation.
    pub const fn tap_only() -> Self {
        Self { head_pause: false, ..Self::new() }
    }
    pub fn recenter(&mut self, pose: Pose) {
        if !pose.ready { return; }
        self.center_horizontal = pose.horizontal; self.center_pitch = pose.pitch;
        self.x = 0; self.y = 0; self.down = 0; self.initialized = true;
    }
    pub fn tap(&mut self, pose: Pose) {
        if self.blocked || !pose.ready { return; }
        if self.phase == Phase::Live {
            self.phase = Phase::Paused; self.pause_pitch = pose.pitch;
        } else {
            self.recenter(pose); self.phase = Phase::Live;
        }
    }
    pub fn step(&mut self, pose: Pose, blocked: bool) -> bool {
        self.blocked = blocked;
        if !pose.ready {
            self.initialized = false;
            if self.phase != Phase::Paused { self.phase = Phase::Waiting; }
            return false;
        }
        // Acquire a resting pose once on startup/recovery, with no user task.
        // Menus, landings and room changes preserve it during ordinary play.
        if !self.initialized { self.recenter(pose); }
        if self.phase == Phase::Waiting { self.phase = Phase::Live; }
        self.x = delta(pose.horizontal, self.center_horizontal);
        self.y = pose.pitch - self.center_pitch;
        if blocked { return false; }
        if self.phase == Phase::Paused {
            if self.head_pause && pose.pitch - self.pause_pitch >= 600 {
                self.recenter(pose); self.phase = Phase::Live;
            }
            return false;
        }
        if self.head_pause && self.y < -2200 {
            self.down += STEP;
            if self.down >= 400 { self.phase = Phase::Paused; self.pause_pitch = pose.pitch; }
            return false;
        }
        self.down = 0;
        true
    }
}
