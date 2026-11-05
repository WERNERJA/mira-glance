//! SDK IMU ownership. Orientation/motion getters claim full IMU; stop releases
//! that claim. Threshold-only apps keep the shared look-up dashboard available.
#[link(wasm_import_module = "mira")]
extern "C" {
    fn imu_threshold(above_cdeg:i32,below_cdeg:i32)->i32;
    fn imu_stop();
}

/// 0 below, 1 above, -1 asleep/unavailable, -2 full IMU requested by the app/phone.
pub unsafe fn threshold_state()->i32 { imu_threshold(2_700,2_400) }

#[allow(dead_code)]
pub unsafe fn stop() { imu_stop(); }
