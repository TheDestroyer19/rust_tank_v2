/// Handles communication between time sensitive components (I2C bus)
/// and the rest of the program

use std::sync::mpsc;
use std::sync::mpsc::{Receiver, TryRecvError, Sender};
use std::thread;
use std::thread::{JoinHandle};
use std::time::{SystemTime, Duration};

use i2cdev::linux::LinuxI2CError;

use mpu6050;
use pca9685::PCA9685;
use mpu6050::MPU6050;

const PWM_FREQ: f32 = 120.0;

pub struct RTHandle {
    pub rx: Receiver<Result<SensorState, LinuxI2CError>>,
    pub tx: Sender<RTCommand>,
    handle: JoinHandle<()>,
}

pub enum RTCommand {
    SetPwm {
        pwm: u8,
        on: u16,
        off: u16,
    },
    SetPwmOff(u8),
    SetPwmOn(u8),
    StopAllMotors(),
    End,
}

pub struct SensorState  {
    time: SystemTime,
    gyro: (f32, f32, f32),
    accel: (f32, f32, f32),
    temp: f32,
    //TODO should PWM state be included?
}

pub fn initialize() -> Result<RTHandle, LinuxI2CError> {
    // initialize PWM hardware
    let mut pca = PCA9685::default()?;
    pca.set_all_pwm_off()?;
    pca.set_pwm_freq(PWM_FREQ)?;
    // initialize MPU hardware
    let mut mpu = MPU6050::new(0x68)?;
    mpu.set_accel_range(mpu6050::ACCEL_RANGE_2G)?;
    mpu.set_gyro_range(mpu6050::GYRO_RANGE_250DEG)?;
    // setup communication channels
    let (rt_tx, rx) = mpsc::channel();
    let (tx, rt_rx) = mpsc::channel();
    // setup the real time thread
    //TODO ensure that this thread runs when its asked to.
    let handle = thread::spawn(|| real_time_loop(pca, mpu, rt_tx, rt_rx));

    Ok(RTHandle {
        rx, tx, handle: handle,
    })
}

fn real_time_loop(mut pca: PCA9685, mut mpu: MPU6050,
                  tx: Sender<Result<SensorState, LinuxI2CError>>,
                  rx: Receiver<RTCommand>) {
    let target_interval = Duration::new(0,55555555);
    loop {
        let time = SystemTime::now();
        if let Err(_) = tx.send(collect_data(&mut mpu, &mut pca, time.clone())) {
            return; //Main thread ended / dropped the handle
        }
        if let Err(e) = match rx.try_recv() {
            Err(TryRecvError::Empty) => Ok(()), //nothing to do
            Err(TryRecvError::Disconnected) => return, //Main thread ended / dropped the handle
            Ok(RTCommand::SetPwm {pwm, on, off}) => pca.set_pwm(pwm, on, off),
            Ok(RTCommand::SetPwmOff(pwm)) => pca.set_pwm_off(pwm),
            Ok(RTCommand::SetPwmOn(pwm)) => pca.set_pwm_on(pwm),
            Ok(RTCommand::StopAllMotors()) => pca.set_all_pwm_off(),
            Ok(RTCommand::End) => return, //Main thread asked us to stop
        } {
            if let Err(_) = tx.send(Err(e)) { return; }
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

fn collect_data(mpu: &mut MPU6050, pca: &mut PCA9685, time: SystemTime) -> Result<SensorState, LinuxI2CError> {
    let accel = mpu.get_accel_data(true)?;
    let gyro = mpu.get_gyro_data()?;
    let temp = mpu.get_temp()?;
    Ok(SensorState {
        accel, time, gyro, temp,
    })
}