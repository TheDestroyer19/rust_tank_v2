pub mod messages;

use std::sync::mpsc;
use std::sync::mpsc::{Receiver, TryRecvError, Sender};
use std::thread;
use std::thread::{JoinHandle};
use std::collections::VecDeque;
use std::net::{TcpListener, TcpStream, ToSocketAddrs};
use std::io;
use std::io::{BufReader, BufRead, Write};

use ::hardware_interface::SensorState;

use serde_json;

use self::messages::*;

pub struct TcpInterface {
    command_queue: VecDeque<Command>,
    rx: Receiver<Command>,
    tx: Sender<Response>,
}

impl TcpInterface {
    pub fn new<A: ToSocketAddrs>(addr: A) -> Result<TcpInterface, io::Error> {
        let listener = TcpListener::bind(addr)?;
        let (tx, handler_rx) = mpsc::channel();
        let handler_rx_loopback = tx.clone();
        let (handler_tx, rx) = mpsc::channel();

        //Start tcp thread
        let join_handle = thread::spawn(move || tcp_handler(listener, handler_rx, handler_rx_loopback, handler_tx));

        let command_queue = VecDeque::new();
        return Ok(TcpInterface { command_queue, rx, tx})
    }
    pub fn next_command(&mut self) -> Option<Command> {
        while let Ok(c) = self.rx.try_recv() {
            if let Command::StopNow = c {
                return Some(Command::StopNow);
            }
            self.command_queue.push_back(c);
        }

        self.command_queue.pop_front()
    }
    pub fn send_state(&mut self, sensor_state: SensorState) {
        self.tx.send(Response::SensorState(sensor_state)).expect("TCP send channel broken");
    }
}

const HELP_PROMPT: &str = "\
Available commands:
  help      print this text
  stopnow   stop the tank immediately
";

fn tcp_handler(listener: TcpListener, rx: Receiver<Response>, rx_loopback: Sender<Response>, tx: Sender<Command>) {
    //pack our rx device into a Arc<Mutex>
    use std::sync::{Arc, Mutex};
    let rx = Arc::new(Mutex::new(rx));

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut writer = stream;

                //mask values so that these can be 'owned' by spawned threads and be used
                //by the next
                let mut rx = Arc::clone(&rx);
                let tx = tx.clone();
                let rx_loopback = rx_loopback.clone();

                let read_handle = thread::spawn( move || {
                    loop {
                        let mut buff= String::new();
                        match reader.read_line(&mut buff) {
                            Err(e) => eprintln!("Error found: {}", e),
                            Ok(0) => {
                                eprintln!("EOF reached");
                                return;
                            },
                            Ok(_) => {
                                //echo to error
                                eprintln!("received: \"{}\"", &buff);
                                buff = buff.trim().to_lowercase();

                                //check command against list
                                match &buff {
                                    x if x == "help" => rx_loopback.send(Response::Ok)
                                        .and_then(|()| rx_loopback.send(Response::UserMsg(String::from(HELP_PROMPT)))),
                                    x if x == "stopnow" => {
                                        tx.send(Command::StopNow).unwrap();
                                        rx_loopback.send(Response::Ok)
                                    },
                                    _ => rx_loopback.send(Response::BadCommand)
                                        .and_then(|()| rx_loopback.send(Response::UserMsg(format!("Invalid command \"{}\"\n", buff))))
                                        .and_then(|()| rx_loopback.send(Response::UserMsg(String::from(HELP_PROMPT)))),
                                }.expect("TCP send channel broken");
                            },
                        }
                    }
                });
                let write_handle = thread::spawn( move || {
                    //since we should be the 'only' thread using the rx, we will mask it with the unwrapped version
                    let rx = rx.lock().unwrap();
                    'write_loop: loop {
                        let r = rx.recv().expect("TCP send channel broken");
                        let msg = match r {
                            Response::UserMsg(s) => s,
                            r => serde_json::to_string(&r).unwrap(),
                        };
                        if let Err(e) = write!(&mut writer, "{}\n", msg) {
                            eprintln!("TCP error: {}", e);
                            break 'write_loop;
                        }
                    }
                });

                if let Err(_) = read_handle.join() {
                    eprintln!("TCP Read thread panicked");
                }
                if let Err(_) = write_handle.join() {
                    eprintln!("TCP Read thread panicked");
                }
            },
            Err(e) => {
                eprintln!("{:?}", e);
            }
        }
    }
}