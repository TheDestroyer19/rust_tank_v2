extern crate mpu6050;
extern crate floating_duration;

use std::io;
//use std::time::Duration;
use std::time::SystemTime;

//use floating_duration::{TimeAsFloat};

use mpu6050::MPU6050;

pub struct Sensors {
    mpu: MPU6050,
    update_time: SystemTime,
    accel: (f32, f32, f32), //Last accel value
    roll: f32,
    pitch: f32,
    gyro: (f32, f32, f32), //Last Gyro value
}

impl Sensors {
    pub fn new() -> io::Result<Sensors> {
        let mut m = MPU6050::new(0x68)?;
        m.set_accel_range(mpu6050::ACCEL_RANGE_2G)?;
        m.set_gyro_range(mpu6050::GYRO_RANGE_250DEG)?;
        Ok(Sensors {
            mpu: m,
            update_time: SystemTime::now(),
            accel: (0.0, 0.0, 0.0),
            roll: 0.0,
            pitch: 0.0,
            gyro: (0.0, 0.0, 0.0)
        })
    }

    pub fn update(&mut self) -> io::Result<()> {
        //get raw data
        let accel = self.mpu.get_accel_data(true)?;
        let gyro = self.mpu.get_gyro_data()?;
        //let dt = self.update_time.elapsed()
        //    .unwrap_or(Duration::from_millis(20))
        //    .as_fractional_secs();
        self.accel = accel;
        self.gyro = gyro;
        self.update_time = SystemTime::now();
        
        //calulate a normalized version of the accel vector
        let (x,y,z) = accel;

        self.roll = y.atan2(z);
        self.pitch = (-x / (y * self.roll.sin() + z * self.roll.cos())).atan();

        Ok(())
    }

    pub fn get_accel(&self) -> (f32, f32, f32) {
        self.accel
    }

    pub fn get_pitch(&self) -> f32 {
        self.pitch
    }

    pub fn get_roll(&self) -> f32 {
        self.roll
    }

    pub fn get_gyro(&self) -> (f32, f32, f32) {
        self.gyro
    }
}