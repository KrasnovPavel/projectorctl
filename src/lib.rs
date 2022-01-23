#![feature(slice_as_chunks)]

use log::warn;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serialport::posix::TTYPort;
use serialport::{DataBits, FlowControl, Parity, SerialPortSettings, StopBits};
use std::fmt::{Display, Formatter};
use std::io::{Read, Write};
use std::path::Path;
use std::time::Duration;
use structopt::StructOpt;
use Command::*;
use SubCommand::*;

#[derive(StructOpt, Serialize, Deserialize, JsonSchema, Debug, Clone)]
#[serde(tag = "State")]
pub enum SubCommand {
    Up,
    Down,
    Status,
}

#[derive(StructOpt, Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub enum Command {
    Power(SubCommand),
    Eco(SubCommand),
    Brightness(SubCommand),
    Volume(SubCommand),
    Mute(SubCommand),
    Source(SubCommand),
    LampTime,
}

#[derive(Serialize, Deserialize, Debug, JsonSchema)]
pub enum Reply {
    State(bool),
    ValueU8(u8),
    ValueU32(u32),
}

#[derive(Serialize, Deserialize, Debug, JsonSchema)]
pub enum ControllerErr {
    SerialPortError,
    PowerIsDown,
    UnsupportedCommand,
}

impl Display for Reply {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Reply::State(v) => write!(f, "{}", v),
            Reply::ValueU8(v) => write!(f, "{}", v),
            Reply::ValueU32(v) => write!(f, "{}", v),
        }
    }
}

impl Command {
    pub fn is_readable(&self) -> bool {
        match &self {
            Power(Status) => true,
            Eco(Status) => true,
            Brightness(Status) => true,
            Volume(Status) => true,
            Mute(Status) => true,
            Source(Status) => true,
            LampTime => true,
            _ => false,
        }
    }
}

pub struct Controller(TTYPort);

impl Controller {
    pub fn new(path: &Path) -> Result<Controller, ControllerErr> {
        let settings = SerialPortSettings {
            baud_rate: 115200,
            data_bits: DataBits::Eight,
            flow_control: FlowControl::None,
            parity: Parity::None,
            stop_bits: StopBits::One,
            timeout: Duration::new(2, 0),
        };
        match TTYPort::open(path, &settings) {
            Ok(t) => Ok(Controller(t)),
            Err(_) => Err(ControllerErr::SerialPortError),
        }
    }

    pub fn read(&mut self, command: &Command) -> Result<Reply, ControllerErr> {
        let power_state =
            parse_state(self.tty_send("\x07\x14\x00\x05\x00\x34\x00\x00\x11\x00\x5E")?);
        if let Power(Status) = command {
            return Ok(power_state);
        };
        if let Reply::State(false) = power_state {
            return Err(ControllerErr::PowerIsDown);
        };
        match command {
            Eco(Status) => Ok(parse_eco_state(
                self.tty_send("\x07\x14\x00\x05\x00\x34\x00\x00\x11\x10\x6E")?,
            )),
            Brightness(Status) => Ok(parse_value_u8(
                self.tty_send("\x07\x14\x00\x05\x00\x34\x00\x00\x12\x03\x62")?,
            )),
            Volume(Status) => Ok(parse_value_u8(
                self.tty_send("\x07\x14\x00\x05\x00\x34\x00\x00\x14\x03\x64")?,
            )),
            Mute(Status) => Ok(parse_state(
                self.tty_send("\x07\x14\x00\x05\x00\x34\x00\x00\x14\x00\x61")?,
            )),
            Source(Status) => Ok(parse_value_u8(
                self.tty_send("\x07\x14\x00\x05\x00\x34\x00\x00\x13\x01\x61")?,
            )),
            LampTime => Ok(parse_value_u32(
                self.tty_send("\x07\x14\x00\x05\x00\x34\x00\x00\x15\x01\x63")?,
            )),
            _ => Err(ControllerErr::UnsupportedCommand),
        }
    }

    pub fn write(&mut self, command: &Command) -> Result<(), ControllerErr> {
        let res = match command {
            Power(Up) => self.tty_send("\x06\x14\x00\x04\x00\x34\x11\x00\x00\x5D"),
            Power(Down) => self.tty_send("\x06\x14\x00\x04\x00\x34\x11\x01\x00\x5E"),
            Source(Up) => self.tty_send("\x06\x14\x00\x04\x00\x34\x13\x01\x03\x63"),
            Source(Down) => self.tty_send("\x06\x14\x00\x04\x00\x34\x13\x01\x07\x67"),
            Eco(Up) => self.tty_send("\x06\x14\x00\x04\x00\x34\x11\x10\x03\x70"),
            Eco(Down) => self.tty_send("\x06\x14\x00\x04\x00\x34\x11\x10\x02\x6F"),
            Volume(Up) => self.tty_send("\x06\x14\x00\x04\x00\x34\x14\x01\x00\x61"),
            Volume(Down) => self.tty_send("\x06\x14\x00\x04\x00\x34\x14\x02\x00\x62"),
            Mute(Up) => self.tty_send("\x06\x14\x00\x04\x00\x34\x14\x00\x01\x61"),
            Mute(Down) => self.tty_send("\x06\x14\x00\x04\x00\x34\x14\x00\x00\x60"),
            Brightness(Up) => self.tty_send("\x06\x14\x00\x04\x00\x34\x12\x03\x01\x62"),
            Brightness(Down) => self.tty_send("\x06\x14\x00\x04\x00\x34\x12\x03\x00\x61"),
            _ => Err(ControllerErr::UnsupportedCommand),
        };
        match res {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }
    }

    fn tty_send(&mut self, command: &str) -> Result<Vec<u8>, ControllerErr> {
        let tty = &mut self.0;
        if let Err(e) = tty.write(command.as_ref()) {
            warn!("Can't write command to tty {:?}", e);
            return Err(ControllerErr::SerialPortError);
        }
        let mut serial_buf: Vec<u8> = vec![0; 5];
        if let Err(e) = tty.read(serial_buf.as_mut_slice()) {
            warn!("Can't read first 5 bytes from tty {:?}", e);
            return Err(ControllerErr::SerialPortError);
        }
        print!("Tty response: {:?}", serial_buf);
        serial_buf.resize((serial_buf[3] + 1) as usize, 0);
        if let Err(e) = tty.read(serial_buf.as_mut_slice()) {
            warn!("Can't read last bytes from tty {:?}", e);
            return Err(ControllerErr::SerialPortError);
        }
        println!("{:?}", serial_buf);
        Ok(serial_buf)
    }
}

fn parse_state(data: Vec<u8>) -> Reply {
    Reply::State(data[2] > 0)
}

fn parse_eco_state(data: Vec<u8>) -> Reply {
    Reply::State(data[2] == 3)
}

fn parse_value_u8(data: Vec<u8>) -> Reply {
    Reply::ValueU8(data[2])
}

fn parse_value_u32(mut data: Vec<u8>) -> Reply {
    data.reverse();
    let d = data.as_chunks().0[0];
    Reply::ValueU32(u32::from_be_bytes(d))
}
