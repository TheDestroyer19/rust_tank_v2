use std::time::{Duration, SystemTime};

use floating_duration::TimeAsFloat;

use super::real_time::{RawSensorState, Vec3};

use std::f32::consts::PI;


#[derive(Serialize, Deserialize)]
pub struct SensorState {
    /// The time between the most recent update and the previous
    duration: Duration,
    /// Time at which this sensor state was last updated.
    time: SystemTime,
    raw_state: RawSensorState,
    roll: f32,
    yaw: f32,
    pitch: f32,
}

impl Default for SensorState {
    fn default() -> SensorState {
        SensorState {
            time: SystemTime::now(),
            duration: Duration::default(),
            raw_state: RawSensorState::default(),
            roll: 0.0,
            yaw: 0.0,
            pitch: 0.0,
        }
    }
}

impl SensorState {
    pub fn update(&mut self, new_state: RawSensorState) {
        //TODO do processing on state
        //TODO consider rolling average for most values.
        let dt = new_state.time.duration_since(self.raw_state.time)
            .unwrap_or(Duration::new(0, 16666667));
        let angles = new_state.orientation.clone();
        self.yaw = angles.x;
        self.pitch = angles.z;
        self.roll = angles.y;
        self.duration = dt;
        self.time = new_state.time.clone();
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

    pub fn time(&self) -> &SystemTime {
        &self.time
    }

    /// Returns the value from the gyro after conversion into deg/s
    /// values are listed in x,y,z order
    pub fn gyro(&self) -> Vec3 {
        self.raw_state.gyro
    }

    /// Returns the value from the accelerometer after conversion into meters/s^2
    /// values are listed in x,y,z order
    pub fn accel(&self) -> Vec3 {
        self.raw_state.accel
    }
}