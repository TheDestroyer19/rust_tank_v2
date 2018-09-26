extern crate pca9685;
extern crate mpu6050;
extern crate termion;
extern crate floating_duration;

use std::thread;
use std::time::Duration;
use std::f32::consts::PI;

use termion::event::Key;

mod motors;
use motors::Motors;
mod sensors;
use sensors::Sensors;
mod hw_tests;
mod terminal;
use terminal::{ InputError};

fn main() {
    //initialize hardware
    pca9685::software_reset().unwrap();
    let mut motors = Motors::new().unwrap();
    motors.set_turret(0).unwrap();
    let mut sensors = Sensors::new().unwrap();
    

    match run(&mut motors, &mut sensors) {
        Err(e) => println!("{}", e),
        _ => (),
    };
    //hw_tests::run_all_tests(&mut motors);
    motors.stop().unwrap();
}

fn run(motors: &mut Motors, sensors: &mut Sensors) -> std::io::Result<()> {
    let (mut input, mut output) = terminal::new()?;

    output.draw_static()?;

    let mut speed = 0.0;
    let mut turn = 0.0;
    let mut degrees = 0;

    loop {
        match match input.next() {
            Ok(Some(k)) => match k {
                Key::Char('x') => break,
                Key::Char('w') => {
                        speed += 0.25;
                        if speed > 1.0 {speed = 1.0; }
                        update_drive(motors, speed, turn)
                    },
                Key::Char('s') => {
                        speed -= 0.25;
                        if speed < -1.0 {speed = -1.0; }
                        update_drive(motors, speed, turn)
                    },
                Key::Char('a') => {
                        turn -= 0.25;
                        if turn < -1.0 {turn = -1.0; }
                        update_drive(motors, speed, turn)
                    },
                Key::Char('d') => {
                        turn += 0.25;
                        if turn > 1.0 {turn = 1.0; }
                        update_drive(motors, speed, turn)
                    },
                Key::Char('q') => {
                    degrees -= 5;
                    if degrees < -90 {degrees = -90; }
                    motors.set_turret(degrees)
                },
                Key::Char('e') => {
                    degrees += 5;
                    if degrees > 90 {degrees = 90; }
                    motors.set_turret(degrees)
                },
                _ => Ok(()),
            },
            Ok(None) => Ok(()),
            Err(InputError::Disconnected) => break,
        } {
            Ok(_) => (),
            Err(e) => { output.print_error(e)? },
        };
        match sensors.update() {
            Ok(_) => (),
            Err(e) => { output.print_error(e)? },
        }
        output.draw_motors(speed, turn, degrees)?;
        output.draw_sensors(
            sensors.get_accel(), sensors.get_gyro(),
            sensors.get_pitch() * 180.0 / PI, sensors.get_roll() * 180.0 / PI,
        )?;
        thread::sleep(Duration::from_millis(10));
    }
    
    Ok(())
}

fn update_drive(motors: &mut Motors, speed: f32, turn: f32) -> std::io::Result<()> {
    let mut ls = speed;
    let mut rs = speed;
    //TODO use turn
    let turn_amount = speed * turn.abs();
    if turn < 0.0 {ls -= turn_amount; }
    else if turn > 0.0 {rs -= turn_amount; }
    if speed == 0.0 {
        ls = turn;
        rs = -turn;
    }
    motors.set_drive_left(ls)?;
    motors.set_drive_right(rs)?;
    Ok(())
}