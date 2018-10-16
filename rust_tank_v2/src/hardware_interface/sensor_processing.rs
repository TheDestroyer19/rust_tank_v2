use std::time::Duration;

use floating_duration::TimeAsFloat;

use super::real_time::RawSensorState;

#[derive(Default)]
pub struct SensorState {
    /// The time between the most recent update and the previous
    duration: Duration,
    raw_state: RawSensorState,
    roll: f32,
    yaw: f32,
    pitch: f32,
}

impl SensorState {
    pub fn update(&mut self, new_state: RawSensorState) {
        //TODO do processing on state
        let dt = new_state.time.duration_since(self.raw_state.time)
            .unwrap_or(Duration::new(0, 16666667));
        let (x, y, z) = new_state.accel;
        self.roll = y.atan2(z);
        self.pitch = (-x / (y * self.roll.sin() + z * self.roll.cos())).atan();
        self.yaw += new_state.gyro.2 * dt.as_fractional_secs() as f32;
        self.duration = dt;
        self.raw_state = new_state;
    }

    pub fn pitch(&self) -> f32 {
        self.pitch
    }

    pub fn roll(&self) -> f32 {
        self.roll
    }

    pub fn yaw(&self) -> f32 {
        self.yaw
    }

    pub fn duration(&self) -> &Duration {
        &self.duration
    }

    /// Returns the value from the gyro after conversion into deg/s
    /// values are listed in x,y,z order
    pub fn gyro(&self) -> (f32, f32, f32) {
        self.raw_state.gyro
    }

    /// Returns the value from the accelerometer after conversion into meters/s^2
    /// values are listed in x,y,z order
    pub fn accel(&self) -> (f32, f32, f32) {
        self.raw_state.accel
    }
}