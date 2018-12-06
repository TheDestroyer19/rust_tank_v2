use floating_duration::TimeAsFloat;

use super::sensor_processing::SensorState;
use super::real_time::RTCommand;

const BIAS: f32 = 0.001;

const MOT_LPWM: u8 = 15;
const MOT_LA: u8 = 13;
const MOT_LB: u8 = 14;
const MOT_RPWM: u8 = 10;
const MOT_RA: u8 = 11;
const MOT_RB: u8 = 12;

/// Data storage for the PID controller
pub struct DrivePid {
    target_power: f32,
    target_deg_per_s: f32,
    //pid variables
    //see http://robotsforroboticists.com/pid-control/
    prev_error: f32,
    integral: f32,
    k_porportional: f32,
    k_integral: f32,
    k_derivative: f32,
    output: f32,
}

impl DrivePid {
    pub fn new(k_porportional: f32,
               k_integral: f32,
               k_derivative: f32)
            -> DrivePid {
        DrivePid {
            target_power: 0.0,
            target_deg_per_s: 0.0,
            k_porportional,
            k_integral,
            k_derivative,
            prev_error: 0.0,
            integral: 0.0,
            output: 0.0,
        }
    }

    pub fn set_target(&mut self, power: f32, dps: f32) {
        self.target_power = power;
        self.target_deg_per_s = dps;
        //TODO should error & integral be reset?
    }

    pub fn update(&mut self,
                  sensors: &SensorState) {
        let dt = sensors.duration().as_fractional_secs() as f32;
        let actual = sensors.gyro().y;
        let error = self.target_deg_per_s - actual;
        self.integral += error * dt;
        let derivative = (error - self.prev_error) / dt;
        self.output = self.k_porportional * error
            + self.k_integral * self.integral
            + self.k_derivative * derivative + BIAS;
        self.prev_error = error;
    }

    pub fn get_pwm_commands(&self) -> Vec<super::RTCommand> {
        if self.target_power == 0.0 && self.target_deg_per_s == 0.0 {
            return vec![RTCommand::SetPwmOff(MOT_LPWM), RTCommand::SetPwmOff(MOT_RPWM)];
        }
        let mut commands = Vec::with_capacity(6);
        //calculate relative power of each motor, clamped to [-1 to 1]
        let lpow = (self.target_power - self.output).max(-1.0).min(1.0);
        let rpow = (self.target_power + self.output).max(-1.0).min(1.0);

        //setup left motor commands
        commands.push(RTCommand::SetPwmOn(if lpow > 0.0 {MOT_LA} else {MOT_LB}));
        commands.push(RTCommand::SetPwmOff(if lpow > 0.0 {MOT_LB} else {MOT_LA}));
        commands.push(RTCommand::SetPwm{
            pwm: MOT_LPWM, on: 0, off: (4095.0 * lpow.abs()).max(1000.0) as u16,
        });

        //setup right motor commands
        //180 degrees out of phase
        commands.push(RTCommand::SetPwmOn(if rpow > 0.0 {MOT_RA} else {MOT_RB}));
        commands.push(RTCommand::SetPwmOff(if rpow > 0.0 {MOT_RB} else {MOT_RA}));
        let off = (2048.0 + (4095.0 * rpow.abs()).max(1000.0)) as u16 % 4098 ;

        commands.push(RTCommand::SetPwm{
            pwm: MOT_RPWM, on: 2048, off,
        });

        commands
    }
}