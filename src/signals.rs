/// This module defines the signals that API server and the main daemon use to
/// communicate.
use lazy_static::lazy_static;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Mutex;

lazy_static! {
    pub static ref CHANNEL: (Mutex<Sender<Message>>, Mutex<Receiver<Message>>) = {
        let (r, w) = channel();
        (Mutex::new(r), Mutex::new(w))
    };
}

#[derive(Debug)]
pub enum Message {
    /// Signifies that the server should start shutdown sequence and stop.
    Shutdown,
    Start(String),
    Stop(String),
    Restart(String),
}

/// Signal the Daemon process with the required signal.
pub fn signal_daemon(msg: Message) {
    let sender = &CHANNEL.0;
    sender.lock().unwrap().send(msg).expect("Failed to send msg to daemon");
}
