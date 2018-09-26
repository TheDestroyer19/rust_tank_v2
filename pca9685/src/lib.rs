extern crate i2cdev;

use std::thread;
use std::time::Duration;

use i2cdev::core::*;
use i2cdev::linux::{LinuxI2CDevice, LinuxI2CError};

//Board constants
const PCA9685_ADDRESS: u16 = 0x40;
const MODE1: u8         = 0x00;
const MODE2: u8         = 0x01;
//const SUBADR1: u8       = 0x02;
//const SUBADR2: u8       = 0x03;
//const SUBADR3: u8       = 0x04;
const PRESCALE: u8      = 0xFE;
const LED0_ON_L: u8     = 0x06;
const LED0_ON_H: u8     = 0x07;
const LED0_OFF_L: u8    = 0x08;
const LED0_OFF_H: u8    = 0x09;
const ALL_LED_ON_L: u8  = 0xFA;
const ALL_LED_ON_H: u8  = 0xFB;
const ALL_LED_OFF_L: u8 = 0xFC;
const ALL_LED_OFF_H: u8 = 0xFD;

const SWRST: u8 = 0x06;
//const RESTART: u8 = 0x80;
const SLEEP: u8 = 0x10;
const ALLCALL: u8 = 0x01;
//const INVRT: u8   = 0x10;
const OUTDRV: u8  = 0x04;

const I2C_DEV: &str = "/dev/i2c-1";

pub struct PCA9685 {
    dev: LinuxI2CDevice,
}

impl PCA9685 {

    /// Shorthand for asking the device to reset.
    pub fn software_reset() -> Result<(), LinuxI2CError> {
        let mut dev = LinuxI2CDevice::new(I2C_DEV, 0x00)?;
        dev.smbus_write_byte(SWRST)?;
        Ok(())
    }

    pub fn new(addr: u16) -> Result<PCA9685, LinuxI2CError> {
        let mut dev = LinuxI2CDevice::new(I2C_DEV, addr)?;

        dev.smbus_write_byte_data(MODE2, OUTDRV)?;
        dev.smbus_write_byte_data(MODE1, ALLCALL)?;

        thread::sleep(Duration::from_millis(5));

        let mode1 = dev.smbus_read_byte()?;
        let mode1 = mode1 & !SLEEP;
        dev.smbus_write_byte_data(MODE1, mode1)?;
        thread::sleep(Duration::from_millis(5));

        Ok(PCA9685 {dev: dev})
    }
    pub fn set_pwm_freq(&mut self, freq_hz: f32)
                        -> Result<(), LinuxI2CError> {
        let mut prescaleval = 25000000.0; //25MHz
        prescaleval /= 4096.0; //12 bit
        prescaleval /= freq_hz;
        prescaleval -= 1.0;
        //println!("Setting PWM frequency to {} Hz", freq_hz);
        //println!("Estimated pre-scale: {}", prescaleval);
        let prescale = (prescaleval +0.5).floor() as u8;
        //println!("Final pre-scale: {}", prescale);
        let oldmode = self.dev.smbus_read_byte()?;
        let newmode = (oldmode & 0x7F) | 0x10; //sleep
        self.dev.smbus_write_byte_data(MODE1, newmode)?;//go to sleep
        self.dev.smbus_write_byte_data(PRESCALE, prescale)?;
        self.dev.smbus_write_byte_data(MODE1, oldmode)?;
        thread::sleep(Duration::from_millis(5));
        self.dev.smbus_write_byte_data(MODE1, oldmode | 0x80)?;
        Ok(())
    }

    /// Directly set value of a pwm pin
    /// Arguments
    ///    channel: The channel that should be updated with the new values (0..15)
    ///    on: The tick (between 0..4095) when the signal should transition from low to high
    ///    off:the tick (between 0..4095) when the signal should transition from high to low
    /// There's also some special settings for turning the pins fully on or fully off
    ///You can set the pin to be fully on with
    ///    pwm.set_pwm(pin, 4096, 0);
    ///You can set the pin to be fully off with
    ///    pwm.set_pwm(pin, 0, 4096);
    pub fn set_pwm(&mut self, channel: u8, on: u16, off: u16) -> Result<(), LinuxI2CError> {
        self.dev.smbus_write_byte_data(LED0_ON_L+4*channel, (on & 0xFF) as u8)?;
        self.dev.smbus_write_byte_data(LED0_ON_H+4*channel, (on >> 8) as u8)?;
        self.dev.smbus_write_byte_data(LED0_OFF_L+4*channel, (off & 0xFF) as u8)?;
        self.dev.smbus_write_byte_data(LED0_OFF_H+4*channel, (off >> 8) as u8)?;
        Ok(())
    }
    /// Directly set value of all pwm pins
    /// Arguments
    ///    on: The tick (between 0..4095) when the signal should transition from low to high
    ///    off:the tick (between 0..4095) when the signal should transition from high to low
    /// There's also some special settings for turning the pins fully on or fully off
    ///You can set the pin to be fully on with
    ///    pwm.set_all_pwm(4096, 0);
    ///You can set the pin to be fully off with
    ///    pwm.set_all_pwm(0, 4096);
    pub fn set_all_pwm(&mut self, on: u16, off: u16) -> Result<(), LinuxI2CError> {
        self.dev.smbus_write_byte_data(ALL_LED_ON_L, (on & 0xFF) as u8)?;
        self.dev.smbus_write_byte_data(ALL_LED_ON_H, (on >> 8) as u8)?;
        self.dev.smbus_write_byte_data(ALL_LED_OFF_L, (off & 0xFF) as u8)?;
        self.dev.smbus_write_byte_data(ALL_LED_OFF_H, (off >> 8) as u8)?;
        Ok(())
    }

    pub fn set_pwm_off(&mut self, channel: u8) -> Result<(), LinuxI2CError> {
        self.set_pwm(channel, 0, 4096)
    }

    pub fn set_pwm_on(&mut self, channel: u8) -> Result<(), LinuxI2CError> {
        self.set_pwm(channel, 4096, 0)
    }

    pub fn set_all_pwm_off(&mut self) -> Result<(), LinuxI2CError> {
        self.set_all_pwm(0, 4096)
    }
}

impl default for PCA9685 {
    fn default() -> Result<PCA9685, LinuxI2CError> {
        PCA9685::new(PCA9685_ADDRESS)
    }

}