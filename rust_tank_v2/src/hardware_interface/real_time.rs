
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, TryRecvError, Sender};
use std::thread;
use std::thread::{JoinHandle};
use std::time::{SystemTime, Duration};

use i2cdev::linux::{LinuxI2CError, LinuxI2CDevice};

use i2cdev_bno055::{BNO055, BNO055_DEFAULT_ADDR, BNO055OperationMode, BNO055PowerMode};

use pca9685::PCA9685;
use mpu6050::{MPU6050, ACCEL_RANGE_2G, GYRO_RANGE_250DEG};
use i2csensors::{Accelerometer, Gyroscope, Magnetometer, Thermometer, Vec3};

pub const PWM_FREQ: f32 = 120.0;

/// Possible commands for i2d devices
pub enum RTCommand {
    SetPwm {
        pwm: u8,
        on: u16,
        off: u16,
    },
    SetPwmOff(u8),
    SetPwmOn(u8),
    StopAllMotors,
    /// Terminates the real time thread, should NOT be used outside of the close method.
    End,
    //TODO consider creating an enum fof each i2c device individually
}

pub struct RawSensorState {
    pub time: SystemTime,
    pub gyro: Vec3,
    pub accel: Vec3,
    pub mag: Vec3,
    pub orientation: Vec3,
    pub temp: f32,
    //TODO should PWM state be included?
}
impl Default for RawSensorState {
    fn default() -> RawSensorState {
        RawSensorState {
            time: SystemTime::now(),
            gyro: Vec3::zeros(),
            accel: Vec3::zeros(),
            mag: Vec3::zeros(),
            orientation: Vec3::zeros(),
            temp: 0.0,
        }
    }
}

pub fn create() -> Result<(JoinHandle<()>, Sender<RTCommand>, Receiver<Result<RawSensorState, LinuxI2CError>>), LinuxI2CError> {
    // initialize PWM hardware
    let mut pca = PCA9685::default()?;
    pca.set_all_pwm_off()?;
    pca.set_pwm_freq(PWM_FREQ)?;

    // initialize MPU hardware
    //let mut mpu = MPU6050::new(0x68)?;
    //mpu.set_accel_range(ACCEL_RANGE_2G)?;
    //mpu.set_gyro_range(GYRO_RANGE_250DEG)?;
    let bno_dev = LinuxI2CDevice::new("/dev/i2c-1", BNO055_DEFAULT_ADDR)?;
    let mut bno = BNO055::new(bno_dev)?;
    bno.reset()?;
    bno.set_external_crystal(true)?;

    // setup communication channels
    let (rt_tx, rx) = mpsc::channel();
    let (tx, rt_rx) = mpsc::channel();
    // setup the real time thread
    //TODO consider setting system thread priority
    let handle = thread::spawn(|| real_time_loop(pca, bno, rt_tx, rt_rx));

    Ok((handle, tx, rx))
}

fn real_time_loop(mut pca: PCA9685, mut bno: BNO055<LinuxI2CDevice>,
                  tx: Sender<Result<RawSensorState, LinuxI2CError>>,
                  rx: Receiver<RTCommand>) {
    let target_interval = Duration::new(0,16666667);
    loop {
        let time = SystemTime::now();
        if let Err(_) = tx.send(collect_data(&mut bno, &mut pca, time.clone())) {
            return; //Main thread ended / dropped the handle
        }
        'commands: loop {
            if let Err(e) = match rx.try_recv() {
                Err(TryRecvError::Empty) => break 'commands, //nothing to do
                Err(TryRecvError::Disconnected) => return, //Main thread ended / dropped the handle
                Ok(RTCommand::SetPwm {pwm, on, off}) => pca.set_pwm(pwm, on, off),
                Ok(RTCommand::SetPwmOff(pwm)) => pca.set_pwm_off(pwm),
                Ok(RTCommand::SetPwmOn(pwm)) => pca.set_pwm_on(pwm),
                Ok(RTCommand::StopAllMotors) => pca.set_all_pwm_off(),
                Ok(RTCommand::End) => return, //Main thread asked us to stop
            } {
                if let Err(_) = tx.send(Err(e)) { return; } // main dropped its rx
            }
        }
        //Sync
        let elapsed = SystemTime::now().duration_since(time);
        let elapsed = elapsed.unwrap_or(Duration::new(0, 1000));//tiny amount of time
        if target_interval > elapsed {
            thread::sleep(target_interval - elapsed);
        } else {
            thread::sleep(Duration::new(0, 1000000));
        }

    }
}

fn collect_data(bno: &mut BNO055<LinuxI2CDevice>, _pca: &mut PCA9685, time: SystemTime) -> Result<RawSensorState, LinuxI2CError> {
    let orientation = bno.get_euler()?;
    let accel = bno.acceleration_reading()?;
    let gyro = bno.angular_rate_reading()?;
    let mag = bno.magnetic_reading()?;
    let temp = bno.temperature_celsius()?;
    Ok(RawSensorState {
        orientation, accel, mag, time, gyro, temp,
    })
}