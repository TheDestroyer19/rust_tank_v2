pub mod messages;

use std::sync::mpsc;
use std::sync::mpsc::{Receiver, TryRecvError, Sender};
use std::thread;
use std::thread::{JoinHandle};
use std::collections::VecDeque;
use std::net::{TcpListener, TcpStream, ToSocketAddrs};
use std::io;
use std::io::{BufReader, BufWriter};

use self::messages::*;

pub struct TcpInterface {
    commandQueue: VecDeque<Command>,
    rx: Receiver<Command>,
    tx: Sender<Response>,
}

fn create_tcp_handler<A: ToSocketAddrs>(addr: A) -> Result<(JoinHandle<()>, Sender<Response>, Receiver<Command>), io::Error> {
    let listener = TcpListener::bind(addr)?;
    let (tx, handler_rx) = mpsc::channel();
    let (handler_tx, rx) = mpsc::channel();

    //Start tcp thread
    let join_handle = thread::spawn(move || {
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let reader = BufReader::new(&stream);
                    let writer = BufWriter::new(&stream);
                },
                Err(e) => {
                    eprintln!("{:?}", e);
                }
            }
        }
    });

    //return
    return Ok((join_handle, tx, rx))
}