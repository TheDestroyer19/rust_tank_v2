extern crate pca9685;
extern crate mpu6050;
extern crate termion;
extern crate floating_duration;
extern crate i2cdev;

use std::thread;
use std::time::Duration;
use std::f32::consts::PI;

use termion::event::Key;

use real_time_interface::{RTHandle, RTCommand};

mod real_time_interface;
//Old modules below
//mod motors;
//use motors::Motors;
//mod sensors;
//use sensors::Sensors;
//mod hw_tests;
mod terminal;
use terminal::{ InputError};


fn main() {


    //initialize hardware
    let mut interface = real_time_interface::RTHandle::initialize().unwrap();

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
        match match input.next() {
            Ok(Some(k)) => match k {
                Key::Char('x') => break,
                Key::Char('w') => {
                        speed += 0.25;
                        if speed > 1.0 {speed = 1.0; }
                        update_drive(interface, speed, turn)
                    },
                Key::Char('s') => {
                        speed -= 0.25;
                        if speed < -1.0 {speed = -1.0; }
                        update_drive(interface, speed, turn)
                    },
                Key::Char('a') => {
                        turn -= 0.25;
                        if turn < -1.0 {turn = -1.0; }
                        update_drive(interface, speed, turn)
                    },
                Key::Char('d') => {
                        turn += 0.25;
                        if turn > 1.0 {turn = 1.0; }
                        update_drive(interface, speed, turn)
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
                _ => Ok(()),
            },
            Ok(None) => Ok(()),
            Err(InputError::Disconnected) => break,
        } {
            Ok(_) => (),
            Err(e) => { output.print_error(e)? },
        };
        for err in interface.update() {
            output.print_error(err)?;
        }
        output.draw_motors(speed, turn, degrees)?;
        output.draw_sensors(
            interface.raw_state().accel, interface.raw_state().gyro,
            interface.pitch() * 180.0 / PI, interface.roll() * 180.0 / PI,
        )?;
        thread::sleep(Duration::from_millis(10));
    }
    
    Ok(())
}

fn update_drive(interface: &mut RTHandle, speed: f32, turn: f32) -> std::io::Result<()> {
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
    //TODO TEMP CODE, Make this implemented in the interface to the hardware
    // or abstracted as commands
    const MOT_LPWM: u8 = 15;
    const MOT_LA: u8 = 13;
    const MOT_LB: u8 = 14;
    const MOT_RPWM: u8 = 10;
    const MOT_RA: u8 = 11;
    const MOT_RB: u8 = 12;
    if ls > 0.0 {
        interface.send_command(RTCommand::SetPwmOn(MOT_LA));
        interface.send_command(RTCommand::SetPwmOff(MOT_LB));
        interface.send_command(RTCommand::SetPwm{
            pwm: MOT_LPWM,
            on: 0,
            off: (4095.0 * ls.abs()) as u16,
        });
    } else if ls < 0.0 {
        interface.send_command(RTCommand::SetPwmOn(MOT_LB));
        interface.send_command(RTCommand::SetPwmOff(MOT_LA));
        interface.send_command(RTCommand::SetPwm{
            pwm: MOT_LPWM,
            on: 0,
            off: (4095.0 * ls.abs()) as u16,
        });
    } else {
        interface.send_command(RTCommand::SetPwmOff(MOT_LB));
        interface.send_command(RTCommand::SetPwmOff(MOT_LA));
        interface.send_command(RTCommand::SetPwmOff(MOT_LPWM));
    }
    if rs > 0.0 {
        interface.send_command(RTCommand::SetPwmOn(MOT_RA));
        interface.send_command(RTCommand::SetPwmOff(MOT_RB));
        interface.send_command(RTCommand::SetPwm{
            pwm: MOT_RPWM,
            on: 0,
            off: (4095.0 * rs.abs()).round() as u16,
        });
    } else if rs < 0.0 {
        interface.send_command(RTCommand::SetPwmOn(MOT_RB));
        interface.send_command(RTCommand::SetPwmOff(MOT_RA));
        interface.send_command(RTCommand::SetPwm{
            pwm: MOT_RPWM,
            on: 0,
            off: (4095.0 * rs.abs()).round() as u16,
        });
    } else {
        interface.send_command(RTCommand::SetPwmOff(MOT_RB));
        interface.send_command(RTCommand::SetPwmOff(MOT_RA));
        interface.send_command(RTCommand::SetPwmOff(MOT_RPWM));
    }
    //END TEMP CODE
    //motors.set_drive_left(ls)?;
    //motors.set_drive_right(rs)?;
    Ok(())
}