use std::time::{Duration, SystemTime};

use super::real_time::{RawSensorState, Vec3};
use super::RTEvent;

use std::cmp::Ord;
use std::f32::consts::PI;

const SONAR_TOO_CLOSE: f32 = 7.5;
const PITCH_TOO_NEG: f32 = -0.2;
const SONAR_SAMPLES: f32 = 16.0;
const ANGLE_EPSILON: f32 = PI / 32.0;

#[derive(Serialize, Deserialize, Clone)]
pub struct SensorState {
    /// The time between the most recent update and the previous
    duration: Duration,
    /// Time at which this sensor state was last updated.
    time: SystemTime,
    raw_state: RawSensorState,
    roll: f32,
    yaw: f32,
    pitch: f32,
    speed: f32,
    /// Sonar distance in CM
    sonar: (f32, SystemTime),
    target_time: Option<SystemTime>,
    target_angle: Option<f32>,
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
            speed: 0.0,
            sonar: (0.0, SystemTime::now()),
            target_time: None,
            target_angle: None,
        }
    }
}

impl SensorState {
    /// Returns Ok when no issue, Err when sonar is too close
    pub fn update(&mut self, new_state: RawSensorState, speed: f32) -> Option<RTEvent> {
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
        self.speed = speed;

        if self.pitch < PITCH_TOO_NEG {
            return Some(RTEvent::SteepIncline);
        }

        if let Some(target) = self.target_time {
            if self.time >= target {
                self.target_time = None;
                return Some(RTEvent::TargetTimeReached);
            }
        }
        if let Some(angle) = self.target_angle {
            let mut delta = angle - self.yaw;
            if delta < 0.0 { delta += PI * 2.0; }
            if delta > PI { delta = PI * 2.0 - delta; }
            if delta < ANGLE_EPSILON {
                self.target_angle = None;
                return Some(RTEvent::TargetAngleReached);
            }
        }

        None
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

    pub fn speed(&self) -> f32 {
        self.speed
    }

    pub fn clear_target_time(&mut self) {
        self.target_time = None;
    }

    pub fn set_target_time(&mut self, time: SystemTime) {
        self.target_time = Some(time);
    }

    pub fn clear_target_angle(&mut self) {
        self.target_angle = None;
    }

    pub fn set_target_angle(&mut self, radians: f32) {
        self.target_angle = Some(radians);
    }

    pub fn set_sonar(&mut self, sonar: (f32, SystemTime)) -> Option<RTEvent> {
        //self.sonar.0 = self.sonar.0 + sonar.0 / SONAR_SAMPLES - self.sonar.0 / SONAR_SAMPLES;
        self.sonar.0 = (self.sonar.0 + sonar.0) / 2.0;
        self.sonar.1 = sonar.1;
        if self.sonar.0 < SONAR_TOO_CLOSE {
            Some(RTEvent::SonarProximity)
        } else {
            None
        }
    }

    pub fn sonar(&self) -> f32 {
        self.sonar.0
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