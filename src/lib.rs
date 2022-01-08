#![feature(slice_as_chunks)]

use structopt::StructOpt;
use serialport::{SerialPortSettings, DataBits, FlowControl, Parity, StopBits};
use serialport::posix::TTYPort;
use std::path::Path;
use std::time::Duration;

#[derive(StructOpt, Debug)]
pub enum SubCommand {
    Up,
    Down,
    Status,
}

#[derive(StructOpt, Debug)]
pub enum Command {
    Power(SubCommand),
    Eco(SubCommand),
    Brightness(SubCommand),
    Volume(SubCommand),
    Mute(SubCommand),
    Source(SubCommand),
    LampTime,
}

#[derive(Debug)]
pub enum Reply {
    State(bool),
    ValueU8(u8),
    ValueU32(u32),
}

#[derive(Debug)]
pub enum ControllerErr {
    SerialPortError,
    UnsupportedCommand,
}

impl Display for Reply
{
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

use Command::*;
use SubCommand::*;
use std::io::{Write, Read};
use std::fmt::{Display, Formatter};

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
        match command {
            Power(Status) => Ok(parse_state(self.tty_read("\x07\x14\x00\x05\x00\x34\x00\x00\x11\x00\x5E")?)),
            Eco(Status) => Ok(parse_eco_state(self.tty_read("\x07\x14\x00\x05\x00\x34\x00\x00\x11\x10\x6E")?)),
            Brightness(Status) => Ok(parse_value_u8(self.tty_read("\x07\x14\x00\x05\x00\x34\x00\x00\x12\x03\x62")?)),
            Volume(Status) => Ok(parse_value_u8(self.tty_read("\x07\x14\x00\x05\x00\x34\x00\x00\x14\x03\x64")?)),
            Mute(Status) => Ok(parse_state(self.tty_read("\x07\x14\x00\x05\x00\x34\x00\x00\x14\x00\x61")?)),
            Source(Status) => Ok(parse_value_u8(self.tty_read("\x07\x14\x00\x05\x00\x34\x00\x00\x13\x01\x61")?)),
            LampTime => Ok(parse_value_u32(self.tty_read("\x07\x14\x00\x05\x00\x34\x00\x00\x15\x01\x63")?)),
            _ => Err(ControllerErr::UnsupportedCommand),
        }
    }

    pub fn write(&mut self, command: &Command) -> Result<(), ControllerErr> {
        match command {
            Power(Up) => self.tty_write("\x06\x14\x00\x04\x00\x34\x11\x00\x00\x5D"),
            Power(Down) => self.tty_write("\x06\x14\x00\x04\x00\x34\x11\x01\x00\x5E"),
            Source(Up) => self.tty_write("\x06\x14\x00\x04\x00\x34\x13\x01\x03\x63"),
            Source(Down) => self.tty_write("\x06\x14\x00\x04\x00\x34\x13\x01\x07\x67"),
            Eco(Up) => self.tty_write("\x06\x14\x00\x04\x00\x34\x11\x10\x03\x70"),
            Eco(Down) => self.tty_write("\x06\x14\x00\x04\x00\x34\x11\x10\x02\x6F"),
            Volume(Up) => self.tty_write("\x06\x14\x00\x04\x00\x34\x14\x01\x00\x61"),
            Volume(Down) => self.tty_write("\x06\x14\x00\x04\x00\x34\x14\x02\x00\x62"),
            Mute(Up) => self.tty_write("\x06\x14\x00\x04\x00\x34\x14\x00\x01\x61"),
            Mute(Down) => self.tty_write("\x06\x14\x00\x04\x00\x34\x14\x00\x00\x60"),
            Brightness(Up) => self.tty_write("\x06\x14\x00\x04\x00\x34\x12\x03\x01\x62"),
            Brightness(Down) => self.tty_write("\x06\x14\x00\x04\x00\x34\x12\x03\x00\x61"),
            _ => Err(ControllerErr::UnsupportedCommand)
        }
    }

    fn tty_write(&mut self, command: &str) -> Result<(), ControllerErr> {
        match self.0.write(command.as_ref()) {
            Ok(_) => Ok(()),
            Err(_) => Err(ControllerErr::SerialPortError),
        }
    }

    fn tty_read(&mut self, command: &str) -> Result<Vec<u8>, ControllerErr> {
        if let Err(_) = self.0.write(command.as_ref()) {
            return Err(ControllerErr::SerialPortError);
        }
        let mut serial_buf: Vec<u8> = vec![0; 5];
        if let Err(_) = self.0.read(serial_buf.as_mut_slice()) {
            return Err(ControllerErr::SerialPortError);
        }
        serial_buf.resize(serial_buf[3] as usize, 0);
        if let Err(_) = self.0.read(serial_buf.as_mut_slice()) {
            return Err(ControllerErr::SerialPortError);
        }
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
