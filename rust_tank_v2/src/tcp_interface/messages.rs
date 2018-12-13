use std::time::Duration;

use super::super::hardware_interface::SensorState;


#[derive(Debug)]
pub enum Command {
    /// STOP all motors, immediately
    /// This also will clear all pending commands.
    StopNow,
    /// Ask the tank to give the current state of onboard sensors
    // TODO should I split this up into multiple components?
    GetSensorState,
    /// Moves the tank in a strait line, until end condition is met.
    /// speed ranges from -1 to 1. Positive speeds for forward, negative for backward.
    /// Target_yaw is the desired angle in degrees
    Move{speed: f64, target_yaw: Option<f64>, end: Option<EndCondition>},
}

#[derive(Debug)]
pub enum EndCondition {
    Time(Duration),
    AngleReached,
}

#[derive(Serialize)]
pub enum Response {
    /// Command was processed successfully
    Ok,
    /// Command failed to parse
    BadCommand(String),
    /// Current state of sensors
    SensorState(SensorState),
    /// Raw text to be displayed to user
    UserMsg(String),
}