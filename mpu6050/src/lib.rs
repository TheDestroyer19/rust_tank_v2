extern crate i2cdev;

use i2cdev::core::*;
use i2cdev::linux::{LinuxI2CDevice, LinuxI2CError};

const I2C_DEV: &str = "/dev/i2c-1";

const GRAVITY_MS2: f32 = 9.80665;

const ACCEL_SCALE_MODIFIER_2G: f32 = 16384.0;
const ACCEL_SCALE_MODIFIER_4G: f32 = 8192.0;
const ACCEL_SCALE_MODIFIER_8G: f32 = 4096.0;
const ACCEL_SCALE_MODIFIER_16G: f32 = 2048.0;

const GYRO_SCALE_MODIFIER_250DEG: f32 = 131.0;
const GYRO_SCALE_MODIFIER_500DEG: f32 = 65.5;
const GYRO_SCALE_MODIFIER_1000DEG: f32 = 32.8;
const GYRO_SCALE_MODIFIER_2000DEG: f32 = 16.4;

//Pre-defined ranges
pub const ACCEL_RANGE_2G: u8 = 0x00;
pub const ACCEL_RANGE_4G: u8 = 0x08;
pub const ACCEL_RANGE_8G: u8 = 0x10;
pub const ACCEL_RANGE_16G: u8 = 0x18;

pub const GYRO_RANGE_250DEG: u8 = 0x00;
pub const GYRO_RANGE_500DEG: u8 = 0x08;
pub const GYRO_RANGE_1000DEG: u8 = 0x10;
pub const GYRO_RANGE_2000DEG: u8 = 0x18;

//MPU-6050 Registers
const PWR_MGMT_1: u8 = 0x6B;
const PWR_MGMT_2: u8 = 0x6C;
const ACCEL_XOUT0: u8 = 0x3B;
const ACCEL_YOUT0: u8 = 0x3D;
const ACCEL_ZOUT0: u8 = 0x3F;
const TEMP_OUT0: u8 = 0x41;
const GYRO_XOUT0: u8 = 0x43;
const GYRO_YOUT0: u8 = 0x45;
const GYRO_ZOUT0: u8 = 0x47;
const ACCEL_CONFIG: u8 = 0x1C;
const GYRO_CONFIG: u8 = 0x1B;



pub struct MPU6050 {
    dev: LinuxI2CDevice,
}
impl MPU6050 {
    pub fn new(addr: u16) -> Result<MPU6050, LinuxI2CError> {
        let d = LinuxI2CDevice::new(I2C_DEV, addr)?;
        let mut mpu = MPU6050 {dev: d};
        mpu.dev.smbus_write_byte_data(PWR_MGMT_1, 0x00)?;
        Ok(mpu)
    }
    pub fn read_i2c_word(&mut self, register: u8)
                         -> Result<i16, LinuxI2CError> {
        let high = self.dev.smbus_read_byte_data(register)? as i16;
        let low = self.dev.smbus_read_byte_data(register + 1)? as i16;
        let value = (high << 8) | low;
        Ok(value)
    }
    pub fn get_temp(&mut self) -> Result<f32, LinuxI2CError> {
        let raw_tmp = self.read_i2c_word(TEMP_OUT0)? as f32;
        let actual_tmp = (raw_tmp / 340.0) + 36.53;
        Ok(actual_tmp)
    }
    pub fn set_accel_range(&mut self, accel_range: u8) -> Result<(), LinuxI2CError> {
        self.dev.smbus_write_byte_data(ACCEL_CONFIG, 0x00)?;
        self.dev.smbus_write_byte_data(ACCEL_CONFIG, accel_range)?;//TODO enforce use of predefined ranges
        Ok(())
    }
    pub fn read_accel_range(&mut self, raw: bool) -> Result<u8, LinuxI2CError> {
        let raw_data = self.dev.smbus_read_byte_data(ACCEL_CONFIG)?;
        if raw {
            Ok(raw_data)
        } else {
            Ok(match raw_data {
                ACCEL_RANGE_2G => 2,
                ACCEL_RANGE_4G => 4,
                ACCEL_RANGE_8G => 8,
                ACCEL_RANGE_16G => 16,
                _ => 0,
            })
        }
    }
    pub fn get_accel_data(&mut self, g: bool) -> Result<(f32, f32, f32), LinuxI2CError> {
        let mut x = self.read_i2c_word(ACCEL_XOUT0)? as f32;
        let mut y = self.read_i2c_word(ACCEL_YOUT0)? as f32;
        let mut z = self.read_i2c_word(ACCEL_ZOUT0)? as f32;

        let accel_scale_modifier = match self.read_accel_range(true)? {
            ACCEL_RANGE_2G => ACCEL_SCALE_MODIFIER_2G,
            ACCEL_RANGE_4G => ACCEL_SCALE_MODIFIER_4G,
            ACCEL_RANGE_8G => ACCEL_SCALE_MODIFIER_8G,
            ACCEL_RANGE_16G => ACCEL_SCALE_MODIFIER_16G,
            _ => {println!("Unknown accel range - calculated as 2G"); ACCEL_SCALE_MODIFIER_2G}
        };
        x = x / accel_scale_modifier;
        y = y / accel_scale_modifier;
        z = z / accel_scale_modifier;

        if !g {
            x = x * GRAVITY_MS2;
            y = y * GRAVITY_MS2;
            z = z * GRAVITY_MS2;
        }

        Ok((x,y,z))
    }
    pub fn set_gyro_range(&mut self, gyro_range: u8) -> Result<(), LinuxI2CError> {
        self.dev.smbus_write_byte_data(GYRO_CONFIG, 0x00)?;
        self.dev.smbus_write_byte_data(GYRO_CONFIG, gyro_range)?;//TODO enforce use of predefined ranges
        Ok(())
    }
    pub fn read_gyro_range_raw(&mut self) -> Result<u8, LinuxI2CError> {
        self.dev.smbus_read_byte_data(GYRO_CONFIG)
    }
    pub fn read_gyro_range(&mut self) -> Result<u16, LinuxI2CError> {
        Ok(match self.dev.smbus_read_byte_data(GYRO_CONFIG)? {
            GYRO_RANGE_250DEG => 250,
            GYRO_RANGE_500DEG => 500,
            GYRO_RANGE_1000DEG => 1000,
            GYRO_RANGE_2000DEG => 2000,
            _ => 0,
        })
    }
    pub fn get_gyro_data(&mut self) -> Result<(f32, f32, f32), LinuxI2CError> {
        let mut x = self.read_i2c_word(GYRO_XOUT0)? as f32;
        let mut y = self.read_i2c_word(GYRO_YOUT0)? as f32;
        let mut z = self.read_i2c_word(GYRO_ZOUT0)? as f32;

        let gyro_scale_modifier = match self.read_gyro_range_raw()? {
            GYRO_RANGE_250DEG => GYRO_SCALE_MODIFIER_250DEG,
            GYRO_RANGE_500DEG => GYRO_SCALE_MODIFIER_500DEG,
            GYRO_RANGE_1000DEG => GYRO_SCALE_MODIFIER_1000DEG,
            GYRO_RANGE_2000DEG => GYRO_SCALE_MODIFIER_2000DEG,
            _ => {println!("Unknown gyro range - calculated as 250DEG"); GYRO_SCALE_MODIFIER_250DEG}
        };
        x = x / gyro_scale_modifier;
        y = y / gyro_scale_modifier;
        z = z / gyro_scale_modifier;

        Ok((x,y,z))
    }
}