extern crate pca9685;

use std::io;

const PWM_FREQ: f32 = 120.0;

const MOT_LA: u8 = 11;
const MOT_LB: u8 = 12;
const MOT_RA: u8 = 13;
const MOT_RB: u8 = 14;
const MOT_T1: u8 =  0;
const T1_MIN: i32 = 240; //was 120
const T1_MAX: i32 = 1240; //was 620//bigger = closer to lower limit

fn clamp<T>(input: T, min: T, max: T) 
        -> T where T: PartialOrd<T> {
    if min > input { return min; }
    if max < input { return max; }
    input
} 

pub struct Motors {
    dev: pca9685::PCA9685,
    mot_left: f32,
    mot_right: f32,
    turr: i8,
}

impl Motors {
    pub fn new() -> io::Result<Motors> {
        let mut d = pca9685::new_default()?;
        d.set_pwm_freq(PWM_FREQ)?;
        Ok(Motors {dev: d, mot_left: 0.0, mot_right: 0.0, turr: 0})
    }
    
    pub fn stop(&mut self) -> io::Result<()> {
        self.dev.set_all_pwm(4096, 0)?;
        self.mot_left = 0.0;
        self.mot_right = 0.0;
        //Turret will remain at (roughly) the same physical location
        Ok(())
    }
    
    pub fn set_turret(&mut self, degrees: i32) -> io::Result<()> {
        let mut value: i32 = (90 - degrees) * (T1_MAX - T1_MIN) / 180 + T1_MIN;
        value = clamp(value, T1_MIN, T1_MAX);
        self.dev.set_pwm(MOT_T1, 0, value as u16)?;
        self.turr = clamp(degrees, -90, 90) as i8;
        Ok(())
    }
    
    pub fn set_drive_left(&mut self, speed: f32) -> io::Result<()> {
        let speed = clamp(speed, -1.0, 1.0);
        let r = self.set_dc_motor(speed, MOT_LA, MOT_LB);
        self.mot_left = speed;
        r
    }
    
    pub fn set_drive_right(&mut self, speed: f32) -> io::Result<()> {
        let speed = clamp(speed, -1.0, 1.0);
        let r = self.set_dc_motor(speed, MOT_RA, MOT_RB);
        self.mot_right = speed;
        r
    }
    
    fn set_dc_motor(&mut self, speed: f32, p1: u8, p2: u8) -> io::Result<()> {
        //Assuming that speed has been clamped to -1 to 1
        if speed < 0.0 {
            self.dev.set_pwm(p1, 4096, 0)?;
            self.dev.set_pwm(p2, (4095.0 * -speed) as u16, 0)?;
        } else if speed > 0.0 {
            self.dev.set_pwm(p1, (4095.0 * speed) as u16, 0)?;
            self.dev.set_pwm(p2, 4096, 0)?;
        } else {//speed == 0
            //set both pwm to off
            self.dev.set_pwm(p1, 4096, 0)?;
            self.dev.set_pwm(p2, 4096, 0)?;
        }
        Ok(())
    }
}
