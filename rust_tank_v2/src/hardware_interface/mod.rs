/// Handles communication between time sensitive components (I2C bus)
/// and the rest of the program

use std::sync::mpsc::{Receiver, TryRecvError, Sender};
use std::thread::{JoinHandle};

use i2cdev::linux::LinuxI2CError;

mod real_time;
mod sensor_processing;
mod drive_pid;

pub use self::real_time::{RTCommand, RawSensorState};
pub use self::sensor_processing::SensorState;


/// Interface to the real time thread
/// This is currently responsible for managing communication with i2c devices
pub struct RTHandle {
    rx: Receiver<Result<RawSensorState, LinuxI2CError>>,
    tx: Sender<RTCommand>,
    handle: JoinHandle<()>,
    sensor_state: SensorState,
    drive_pid: drive_pid::DrivePid,
}

impl RTHandle {
    /// Instantiates the interface between the non-critital timing
    /// portions of the controller, and the timing critical portions. This sets
    /// up the initial state of the I2C devices, and starts the
    /// thread that controls them.
    pub fn initialize() -> Result<RTHandle, LinuxI2CError> {

        let (handle, tx, rx)
            = real_time::create()?;

        //TODO tune drive
        let drive_pid = drive_pid::DrivePid::new(0.02, 0.05, 0.0);

        Ok(RTHandle {
            rx, tx,
            handle: handle,
            sensor_state: SensorState::default(),
            drive_pid,
        })
    }

    /// Goes through all state updates received, and updates the current state
    /// Returns any IO errors that occured on the other thread since last call to update
    /// Blocks until a message is received
    pub fn update(&mut self) -> Vec<LinuxI2CError> {

        let mut errors = vec![];
        let mut next = match self.rx.recv() {
            Ok(rss) => rss,
            Err(_) => {
                panic!("Real time thread aborted!");
            }
        };
        'queue: loop {
            match next {
                Ok(new_state) => {
                    self.sensor_state.update(new_state);
                    self.drive_pid.update(&self.sensor_state);
                },
                Err(err) => {
                    errors.push(err);
                }
            }
            next = match self.rx.try_recv() {
                Ok(rss) => rss,
                Err(TryRecvError::Empty) => break 'queue,
                Err(TryRecvError::Disconnected) => {
                    panic!("Real time thread aborted!");
                },
            };
        }
        //now let things do updates that aren't retroactive
        for msg in self.drive_pid.get_pwm_commands() {
            self.send_command(msg);
        }
        errors
    }

    /// Sends a command to an I2C device
    fn send_command(&mut self, command: RTCommand) {
        //shouldn't panic unless the other thread terminates
        if let Err(_) =self.tx.send(command) {
            //TODO ensure motors are stopped
            //If the real time thread ends (or drops the communication pipe
            //then we cannot recover, except maybe make the HW enter a safe state.
            panic!("Real time thread aborted!");
        }
    }

    pub fn set_drive(&mut self, power: f32, turn: f32) {
        self.drive_pid.set_target(power, turn);
    }

    pub fn state(&self) -> &SensorState {
        &self.sensor_state
    }

    /// Stops all the motors
    pub fn stop(&mut self) {
        self.drive_pid.set_target(0.0, 0.0);
        self.send_command(RTCommand::StopAllMotors);
    }

    pub fn close(mut self) {
        self.send_command(RTCommand::StopAllMotors);
        self.send_command(RTCommand::End);
        self.handle.join().expect("Real time thread paniced!");
    }
}