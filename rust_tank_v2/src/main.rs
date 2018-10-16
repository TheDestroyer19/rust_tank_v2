extern crate pca9685;
extern crate mpu6050;
extern crate termion;
extern crate floating_duration;
extern crate i2cdev;

use std::thread;
use std::time::Duration;
use std::f32::consts::PI;

use termion::event::Key;


mod terminal;
use terminal::{ InputError};

mod hardware_interface;
use hardware_interface::{RTHandle};
//Old modules below
//mod hw_tests;


fn main() {


    //initialize hardware
    let mut interface = RTHandle::initialize().unwrap();

    match run(&mut interface) {
        Err(e) => println!("{}", e),
        _ => (),
    };

    interface.close();
}

fn run(interface: &mut RTHandle) -> std::io::Result<()> {
    let (mut input, mut output) = terminal::new()?;

    output.draw_static()?;

    let mut speed = 0.0;
    let mut turn = 0.0;
    let mut degrees = 0;

    loop {
        match input.next() {
            Ok(Some(k)) => match k {
                Key::Char('x') => break,
                Key::Char('w') => {
                    speed += 0.25;
                    if speed > 1.0 {speed = 1.0; }
                    interface.set_drive(speed, turn);
                },
                Key::Char('s') => {
                    speed -= 0.25;
                    if speed < -1.0 {speed = -1.0; }
                    interface.set_drive(speed, turn);
                },
                Key::Char('d') => {
                    turn -= 0.5;
                    if turn < -20.0 {turn = -20.0; }
                    interface.set_drive(speed, turn);
                },
                Key::Char('a') => {
                    turn += 0.5;
                    if turn > 20.0 {turn = 20.0; }
                    interface.set_drive(speed, turn);
                },
                Key::Char('z') => {
                    turn = 0.0;
                    speed = 0.0;
                    interface.stop();
                },
                /*Key::Char('q') => {
                    degrees -= 5;
                    if degrees < -90 {degrees = -90; }
                    motors.set_turret(degrees)
                },
                Key::Char('e') => {
                    degrees += 5;
                    if degrees > 90 {degrees = 90; }
                    motors.set_turret(degrees)
                },*/
                _ => (),
            },
            Ok(None) => (),
            Err(InputError::Disconnected) => break,
        }
        for err in interface.update() {
            output.print_error(err)?;
        }
        output.draw_motors(speed, turn, degrees)?;
        output.draw_sensors(
            interface.state().accel(), interface.state().gyro(),
            interface.state().pitch() * 180.0 / PI, interface.state().roll() * 180.0 / PI,
            interface.state().yaw(),
        )?;
        thread::sleep(Duration::from_millis(16));
    }
    
    Ok(())
}
