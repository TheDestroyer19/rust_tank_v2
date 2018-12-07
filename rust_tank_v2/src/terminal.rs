use std::io;
use std::io::{stdin, stdout, Stdout, Write};

use std::fmt::Display;

use std::sync::mpsc;
use std::sync::mpsc::{Receiver, TryRecvError};

use std::thread;

use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use termion::raw::RawTerminal;
use termion::{clear, color, cursor, style};

use super::hardware_interface::Vec3;

pub fn new() -> io::Result<(Input, Output)> {
    let output = Output::new()?;
    let input = Input::new();
    Ok((input, output))
}

pub struct Input {
    rx: Receiver<Key>
}

pub enum InputError {
    Disconnected, 
}

impl Input {
    fn new() -> Input {
        let stdin = stdin();
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let h = stdin.lock();
            for c in h.keys() {
                let k = c.unwrap();//TODO handle errors better
                tx.send(k).unwrap();
            }
        });

        Input {rx}
    }

    pub fn next(&mut self) -> Result<Option<Key>, InputError> {
        match self.rx.try_recv() {
            Ok(k) => Ok(Some(k)),
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => Err(InputError::Disconnected),
        }
    }
}

pub struct Output {
    out: RawTerminal<Stdout>,
    msg_count: u16,
}

impl Output {
    fn new() -> io::Result<Output> {
        let mut out = stdout().into_raw_mode()?;
        //setup output
        write!(out, "{}{}{}",
            clear::All,
            cursor::Goto(1,1),
            cursor::Hide
        )?;
        Ok(Output { out, msg_count: 0 })
    }

    pub fn draw_static(&mut self) -> io::Result<()> {
        write!(self.out, "{}\
            ╔═════════════════════╦════════════════╗\r\n\
            ║{} RustTank v0.1.0     {}║    ax:         ║\r\n\
            ╠═════════════════════╣    ay:         ║\r\n\
            ║Controls             ║    az:         ║\r\n\
            ║ W/S - Change speed  ║    gx:         ║\r\n\
            ║ A/D - Turn Tank     ║    gy:         ║\r\n\
            ║ Q/E - Rotate turret ║    gz:         ║\r\n\
            ║  T  - Run HW tests  ║ pitch:         ║\r\n\
            ║  Z  - Stop motors   ║  roll:         ║\r\n\
            ║  X  - Exit          ║   yaw:         ║\r\n\
            ╠═════════════════════╩════════════════╣\r\n\
            ║ Speed:      Turn:      Turret:       ║\r\n\
            ╚══════════════════════════════════════╝",
            cursor::Goto(1,1), color::Fg(color::Green), style::Reset)?;
        self.out.flush()?;
        Ok(())
    }

    pub fn draw_sensors(&mut self, 
            acc: Vec3,
            gyro: Vec3,
            pitch: f32, roll: f32, yaw: f32) -> io::Result<()> {
        write!(self.out, "{}{:9.3}{}{:9.3}{}{:9.3}", 
            cursor::Goto(31,2), acc.x,
            cursor::Goto(31,3), acc.y,
            cursor::Goto(31,4), acc.z)?;
        write!(self.out, "{}{:9.3}{}{:9.3}{}{:9.3}", 
            cursor::Goto(31,5), gyro.x,
            cursor::Goto(31,6), gyro.y,
            cursor::Goto(31,7), gyro.z)?;
        write!(self.out, "{}{:9.3}{}{:9.3}{}{:9.3}",
            cursor::Goto(31,8), pitch, 
            cursor::Goto(31,9), roll,
            cursor::Goto(31,10), yaw)?;
        self.out.flush()?;
        Ok(())
    }

    pub fn draw_motors(&mut self, 
            speed: f32, turn: f32, turret: i32) 
            -> io::Result<()> {
        write!(self.out, "{}{:5}{}{:5}{}{:5}",
            cursor::Goto(9, 12), speed,
            cursor::Goto(20,12), turn,
            cursor::Goto(33, 12), turret
        )?;
        self.out.flush()?;
        Ok(())
    }

    pub fn print_error<T: Display>(&mut self, e: T) -> io::Result<()> {
        write!(self.out, "{}{}{}{}{}{}", 
            cursor::Goto(3,13 + self.msg_count), clear::CurrentLine, 
            color::Fg(color::Red), style::Bold,
            e, style::Reset)?;
        self.out.flush()?;
        self.msg_count += 1;
        Ok(())
    }
}

impl Drop for Output {
    fn drop(&mut self) {
        //cleanup terminal
        match write!(self.out, "{}{}{}",
            clear::All,
            cursor::Goto(1,1),
            cursor::Show) {
                Ok(_) => (),
                Err(e) => eprintln!("IO Error while dropping output: {:?}", e),
            };
        match self.out.flush() {
            Ok(_) => (),
            Err(e) => eprintln!("IO Error while dropping output: {:?}", e),
        };
    }
}