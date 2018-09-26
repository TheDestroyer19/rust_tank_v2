use std::thread;
use std::time::Duration;

use motors::Motors;

pub fn run_all_tests(motors: &mut Motors) {
    test_turret(motors);
    test_drive(motors);
}

pub fn test_turret(motors: &mut Motors) {
    println!("\nTesting Turret");
    for d in -90..91 {
        motors.set_turret(d).unwrap();
        if d % 15 == 0 {
            println!("{}", d);
            thread::sleep(Duration::from_millis(1000));
        } else {
            thread::sleep(Duration::from_millis(100));
        }
    }
    motors.stop().unwrap();
}

pub fn test_drive(motors: &mut Motors) {
    println!("\nTesting Left Drive Motor");
        for s in -20..21 {
            let speed = s as f32 / 20.0;
            println!("{} ", speed);
            motors.set_drive_left(speed).unwrap();
            thread::sleep(Duration::from_millis(250));
        }
        motors.stop().unwrap();
        println!("\nTesting Right Drive Motor");
        for s in -20..21 {
            let speed = s as f32 / 20.0;
            println!("{} ", speed);
            motors.set_drive_right(speed).unwrap();
            thread::sleep(Duration::from_millis(250));
        }
        motors.stop().unwrap();
}