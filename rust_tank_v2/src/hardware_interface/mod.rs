/// Handles communication between time sensitive components (I2C bus)
/// and the rest of the program

use std::sync::mpsc::{Receiver, TryRecvError, Sender};
use std::thread::{JoinHandle};

use i2cdev::linux::LinuxI2CError;

mod real_time;
mod sensor_processing;
mod drive_pid;

pub use self::real_time::{RTCommand, RTResponse, RawSensorState, Vec3};
pub use self::sensor_processing::SensorState;
use ::tcp_interface::TcpInterface;


/// Interface to the real time thread
/// This is currently responsible for managing communication with i2c devices
pub struct RTHandle {
    rx: Receiver<RTResponse>,
    tx: Sender<RTCommand>,
    i2c_handle: JoinHandle<()>,
    sonar_handle: JoinHandle<()>,
    sensor_state: SensorState,
    drive_pid: drive_pid::DrivePid,
}

pub enum RTEvent {
    /// Something got too close to the front of the tank
    SonarProximity,
    TargetAngleReached,
    TargetTimeReached,
    /// Some non-fatal i2c error
    Err(LinuxI2CError),
}

impl RTHandle {
    /// Instantiates the interface between the non-critital timing
    /// portions of the controller, and the timing critical portions. This sets
    /// up the initial state of the I2C devices, and starts the
    /// thread that controls them.
    pub fn initialize() -> Result<RTHandle, LinuxI2CError> {

        let (i2c_handle, sonar_handle, tx, rx)
            = real_time::create()?;

        //TODO tune drive
        let drive_pid = drive_pid::DrivePid::new(2.0, 1.0, 0.0);

        Ok(RTHandle {
            rx, tx,
            i2c_handle, sonar_handle,
            sensor_state: SensorState::default(),
            drive_pid,
        })
    }

    /// Goes through all state updates received, and updates the current state
    /// Returns any IO errors that occured on the other thread since last call to update
    /// Blocks until a message is received
    pub fn update(&mut self, send_updates: bool, tcp_interface: &mut TcpInterface) -> Vec<RTEvent> {

        let mut events = vec![];
        let mut next = match self.rx.recv() {
            Ok(rss) => rss,
            Err(_) => {
                panic!("Real time thread aborted!");
            }
        };
        'queue: loop {
            match next {
                RTResponse::I2C(Ok(new_state)) => {
                    let event = self.sensor_state.update(new_state, self.drive_pid.target_power());
                    self.drive_pid.update(&self.sensor_state);
                    if send_updates {
                        tcp_interface.send_state(&self.sensor_state);
                    }
                    if let Some(e) = event {
                        events.push(e);
                    }
                },
                RTResponse::I2C(Err(err)) => {
                    events.push(RTEvent::Err(err));
                },
                RTResponse::Sonar(cm, time) => {
                    if let Some(event) = self.sensor_state.set_sonar((cm, time)) {
                        events.push(event);
                    }
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
        events
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

    pub fn sensor_state(&mut self) -> &mut SensorState {
        &mut self.sensor_state
    }

    /// Stops all the motors
    pub fn stop(&mut self) {
        self.drive_pid.set_target(0.0, 0.0);
        self.send_command(RTCommand::StopAllMotors);
    }

    pub fn close(mut self) {
        self.send_command(RTCommand::StopAllMotors);
        self.send_command(RTCommand::End);
        self.i2c_handle.join().expect("Real time I2C thread paniced!");
        self.sonar_handle.join().expect("Real time Sonar thread paniced!");
    }
}

/// Fix for quirk of RPI
pub mod on_export {
    use std::thread::sleep;
    use std::time::Duration;

    const SLEEP_HEURISTIC_MILLIS: u64 = 50;

    /// wait ~50ms after exporting this pin.
    /// if you set_direction *immediately* after
    /// entering this closure, without your
    /// pin having been exported on a *previous*
    /// run, you'll crash.
    pub fn wait() {
        sleep(Duration::from_millis(SLEEP_HEURISTIC_MILLIS))
    }
}