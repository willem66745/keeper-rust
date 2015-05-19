
use std::sync::mpsc::{channel, Sender, Receiver};
use std::thread;
use std::collections;
use std::error::Error;
use std::fmt;

use plugwise;

#[derive(Debug)]
pub enum SerialError {
    ConnectError(String)
}

impl Error for SerialError {
    fn description(&self) -> &str {
        match *self {
            SerialError::ConnectError(ref e) => &e[..]
        }
    }

    fn cause(&self) -> Option<&Error> {
        None
    }
}

impl fmt::Display for SerialError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{}", self.description())
    }
}

enum Command {
    ConnectDevice(Option<Sender<ConnectResponse>>, String),
    ConnectStub,
    Hangup,
    RegisterCircle(String, u64),
}

enum ConnectResponse {
    Ok,
    ConnectionFailed(String),
}

pub struct Serial;

impl Serial {
    fn message_loop(rx: Receiver<Command>) {
        let mut plugwise = None;
        let mut circles = collections::HashMap::new();

        loop {
            let msg = rx.recv()
                        .ok()
                        .expect("BUG: serial receive loop error");

            match msg {
                Command::ConnectDevice(tx, device) => {
                    let new_plugwise = plugwise::plugwise(plugwise::Device::Serial(device.clone()));

                    match new_plugwise {
                        Ok(new_plugwise) => {
                            plugwise = Some(new_plugwise);
                            if let Some(tx) = tx {
                                tx.send(ConnectResponse::Ok)
                                    .ok().expect("unable to send response");
                            }
                        },
                        Err(err) => {
                            if let Some(tx) = tx {
                                tx.send(ConnectResponse::ConnectionFailed(err.description().into()))
                                    .ok().expect("unable to send response");
                            }
                        }
                    }
                },
                Command::ConnectStub => {
                    let new_plugwise = plugwise::plugwise(plugwise::Device::Simulator).ok().expect(
                                    "creating a simulation instance unexpectedly failed!");
                    plugwise = Some(new_plugwise);
                },
                Command::Hangup => break,
                Command::RegisterCircle(alias, mac) => {
                    if let Some(ref plugwise) = plugwise {
                        let circle = plugwise.create_circle(mac);
                        if let Ok(circle) = circle {
                            circles.insert(alias, circle);
                        }
                    }
                },
            }
        }
    }

    pub fn spawn() -> SerialClient {
        let (boot_tx, boot_rx) = channel();

        thread::spawn(move || {
            let (tx, rx) = channel();

            boot_tx.send(tx.clone())
                   .ok()
                   .expect("BUG: bootstrap failed");

            Serial::message_loop(rx);
        });

        let response = boot_rx.recv()
                              .ok()
                              .expect("BUG: bootstrap message expected");

        SerialClient {
            tx: response,
        }
    }
}

#[derive(Clone)]
pub struct SerialClient {
    tx: Sender<Command>,
}

impl SerialClient {
    pub fn connect_stub(&self) {
        self.tx.send(Command::ConnectStub)
               .ok()
               .expect("BUG: serial thread channel error");
    }

    pub fn connect_device(&self, device: &str) -> Result<(), SerialError> {
        let (tx, rx) = channel();

        self.tx.send(Command::ConnectDevice(Some(tx), device.into()))
               .ok()
               .expect("BUG: serial thread channel error");

        let response = rx.recv().ok().expect("BUG: cannot receive answer from serial thread");

        match response {
            ConnectResponse::Ok => Ok(()),
            ConnectResponse::ConnectionFailed(err) => Err(SerialError::ConnectError(err))
        }
    }

    pub fn register_circle(&self, alias: &str, mac: u64) {
        self.tx.send(Command::RegisterCircle(alias.into(), mac))
               .ok()
               .expect("BUG: cannot register circle");
    }

    pub fn hangup(&self) {
        self.tx.send(Command::Hangup)
               .ok()
               .expect("BUG: cannot bring serial thread down");
    }
}


