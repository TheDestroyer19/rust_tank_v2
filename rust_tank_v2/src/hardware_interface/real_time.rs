
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, TryRecvError, Sender};
use std::thread;
use std::thread::sleep;
use std::thread::{JoinHandle};
use std::time::{SystemTime, Duration};

use super::on_export;
use sysfs_gpio::{Direction, Pin, Edge};

use i2cdev::linux::{LinuxI2CError, LinuxI2CDevice};

use i2cdev_bno055::{BNO055, BNO055_DEFAULT_ADDR, BNO055OperationMode};

use pca9685::PCA9685;
use i2csensors::{Accelerometer, Gyroscope, Magnetometer, Thermometer};
use i2csensors::Vec3 as iVec3;

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

/// Values that are sent from the sonar/i2c threads
pub enum RTResponse {
    I2C(Result<RawSensorState, LinuxI2CError>),
    Sonar(f32, SystemTime),//in cm
}

#[derive(Serialize, Deserialize, Default, Copy, Clone)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}
impl From<iVec3> for Vec3 {
    fn from(t: iVec3) -> Vec3 {
        Vec3 {
            x: t.x,
            y: t.y,
            z: t.z
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
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
            gyro: Vec3::default(),
            accel: Vec3::default(),
            mag: Vec3::default(),
            orientation: Vec3::default(),
            temp: 0.0,
        }
    }
}

pub fn create() -> Result<(JoinHandle<()>, JoinHandle<()>, Sender<RTCommand>, Receiver<RTResponse>), LinuxI2CError> {


    // setup communication channels
    let (i2c_tx, rx) = mpsc::channel();
    let sonar_tx = i2c_tx.clone();
    let (tx, i2c_rx) = mpsc::channel();
    // setup the real time thread
    //TODO consider setting system thread priority
    let i2c_handle = thread::spawn(|| rt_i2c_loop(i2c_tx, i2c_rx));
    let sonar_handle = thread::spawn(|| rt_sonar_loop(sonar_tx));

    Ok((i2c_handle, sonar_handle, tx, rx))
}

fn rt_i2c_loop(tx: Sender<RTResponse>,
               rx: Receiver<RTCommand>) {
    let target_interval = Duration::new(0,16666667);

    // initialize PWM hardware
    let mut pca = PCA9685::default().unwrap();
    pca.set_all_pwm_off().unwrap();
    pca.set_pwm_freq(PWM_FREQ).unwrap();

    // initialize MPU hardware
    let bno = LinuxI2CDevice::new("/dev/i2c-1", BNO055_DEFAULT_ADDR).unwrap();
    let mut bno = BNO055::new(bno).unwrap();
    bno.reset().unwrap();
    bno.set_external_crystal(true).unwrap();
    bno.set_mode(BNO055OperationMode::Ndof).unwrap();

    loop {
        let time = SystemTime::now();
        if let Err(_) = tx.send(RTResponse::I2C(collect_data(&mut bno, &mut pca, time.clone()))) {
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
                if let Err(_) = tx.send(RTResponse::I2C(Err(e))) { return; } // main dropped its rx
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
    let orientation = Vec3::from(bno.get_euler()?);
    let accel = Vec3::from(bno.acceleration_reading()?);
    let gyro = Vec3::from(bno.angular_rate_reading()?);
    let mag = Vec3::from(bno.magnetic_reading()?);
    let temp = bno.temperature_celsius()?;
    Ok(RawSensorState {
        orientation, accel, mag, time, gyro, temp,
    })
}

fn rt_sonar_loop(tx: Sender<RTResponse>) {
    let trigger_pin = Pin::new(18);
    let echo_pin = Pin::new(25);
    trigger_pin
        .with_exported(|| {
            echo_pin.with_exported(|| {
                on_export::wait();
                trigger_pin.set_direction(Direction::Out).unwrap();
                echo_pin.set_direction(Direction::In).unwrap();
                echo_pin.set_edge(Edge::BothEdges).unwrap();
                let mut echo_pin_puller = echo_pin.get_poller().unwrap();

                'sonar: loop {
                    //run trigger
                    //println!("Sending trigger");
                    trigger_pin.set_value(1).unwrap();
                    sleep(Duration::from_millis(1));
                    trigger_pin.set_value(0).unwrap();

                    //wait for signal
                    if echo_pin_puller.poll(500).unwrap() == None {
                        //println!("No read - trying again");
                        continue;
                    }
                    let start = SystemTime::now();

                    //wait for end
                    match echo_pin_puller.poll(500) {
                        Ok(Some(_)) => {
                            //calculate how long
                            let end = SystemTime::now();
                            let time = end.duration_since(start).unwrap();

                            //Convert to distance
                            let distance_cm = (time.subsec_nanos() as f32 * 34300.0) / 2000000000.0;
                            //println!("Distance is {} cm", distance_cm);
                            if let Err(_) = tx.send(RTResponse::Sonar(distance_cm, end)) {
                                break 'sonar; //send only fails when the other end hung up
                            }

                            //let sonar sleep a little
                            sleep(Duration::from_micros(10000));
                        },
                        Err(e) => panic!("Something weird happened {:?}", e),
                        Ok(None) => (), //println!("Echo timed out"),
                    }
                }

                Ok(())
            })
        })
        .unwrap();
}