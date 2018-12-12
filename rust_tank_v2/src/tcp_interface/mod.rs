pub mod messages;

use std::sync::mpsc;
use std::sync::mpsc::{Receiver, TryRecvError, Sender};
use std::thread;
use std::thread::{JoinHandle};
use std::collections::VecDeque;
use std::net::{TcpListener, TcpStream, ToSocketAddrs};
use std::io;
use std::io::{BufReader, BufRead, BufWriter};

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
}

fn tcp_handler(listener: TcpListener, rx: Receiver<Response>, rx_loopback: Sender<Response>, tx: Sender<Command>) {
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut writer = BufWriter::new(stream);

                let read_handle = thread::spawn( move || {
                    loop {
                        let mut buff= String::new();
                        match reader.read_line(&mut buff) {
                            Err(e) => eprintln!("Error found: {}", e),
                            Ok(0) => {
                                eprintln!("EOF reached");
                                return;
                            },
                            Ok(_) => eprintln!("{}", buff),
                        }
                    }
                });
                let write_handle = thread::spawn( move || {
                    //TODO
                    return;
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