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

/// Interface to the real time thread
/// This is currently responsible for managing communication with i2c devices
pub struct RTHandle {
    rx: Receiver<Result<SensorState, LinuxI2CError>>,
    tx: Sender<RTCommand>,
    handle: JoinHandle<()>,
    raw_state: SensorState,
    pitch: f32,
    roll: f32,
}

impl RTHandle {
    /// Instantiates the interface between the non-critital timing
    /// portions of the controller, and the timing critical portions. This sets
    /// up the initial state of the I2C devices, and starts the
    /// thread that controls them.
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
            rx, tx,
            handle: handle,
            raw_state: SensorState::default(),
            roll: 0.0,
            pitch: 0.0,
        })
    }

    /// Goes through all state updates received, and updates the current state
    /// Returns any IO errors that occured on the other thread since last call to update
    pub fn update(&mut self) -> Vec<LinuxI2CError> {
        let mut errors = vec![];
        'queue: loop {
            match self.rx.try_recv() {
                Ok(Ok(new_state)) => {
                    //TODO do processing on state
                    let (x, y, z) = new_state.accel;
                    self.roll = y.atan2(z);
                    self.pitch = (-x / (y * self.roll.sin() + z * self.roll.cos())).atan();
                    self.raw_state = new_state;
                },
                Ok(Err(err)) => {
                    errors.push(err);
                },
                Err(TryRecvError::Empty) => break 'queue, //pipe empty
                Err(TryRecvError::Disconnected) => {
                    panic!("Real time thread aborted!");
                },
            }
        }
        errors
    }

    /// Sends a command to an I2C device
    pub fn send_command(&mut self, command: RTCommand) {
        //shouldn't panic unless the other thread terminates
        if let Err(_) =self.tx.send(command) {
            //TODO ensure motors are stopped
            //If the real time thread ends (or drops the communication pipe
            //then we cannot recover, except maybe make the HW enter a safe state.
            panic!("Real time thread aborted!");
        }
    }

    pub fn raw_state(&self) -> &SensorState {
        &self.raw_state
    }

    pub fn pitch(&self) -> f32 {
        self.pitch
    }

    pub fn roll(&self) -> f32 {
        self.roll
    }

    pub fn close(mut self) {
        self.send_command(RTCommand::StopAllMotors);
        self.send_command(RTCommand::End);
        self.handle.join().expect("Real time thread paniced!");
    }
}

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

pub struct SensorState  {
    pub time: SystemTime,
    pub gyro: (f32, f32, f32),
    pub accel: (f32, f32, f32),
    pub temp: f32,
    //TODO should PWM state be included?
}
impl Default for SensorState {
    fn default() -> SensorState {
        SensorState {
            time: SystemTime::now(),
            gyro: (0.0, 0.0, 0.0),
            accel: (0.0, 0.0, 0.0),
            temp: 0.0,
        }
    }
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

fn collect_data(mpu: &mut MPU6050, _pca: &mut PCA9685, time: SystemTime) -> Result<SensorState, LinuxI2CError> {
    let accel = mpu.get_accel_data(true)?;
    let gyro = mpu.get_gyro_data()?;
    let temp = mpu.get_temp()?;
    Ok(SensorState {
        accel, time, gyro, temp,
    })
}