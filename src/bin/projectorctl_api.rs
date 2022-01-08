#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate rocket;

use projectorctl::*;

use std::io::Cursor;
use std::path::Path;
use rocket::{Config, Request};
use std::net::IpAddr;
use std::str::FromStr;
use rocket::http::Status;
use rocket::response::{self, Response, Responder};

pub struct WebReply(Reply);

impl<'r, 'o: 'r> Responder<'r, 'o>  for WebReply {
    fn respond_to(self, _: &'r Request<'_>) -> response::Result<'o> {
        let body = match self.0 {
            Reply::State(b) => format!("{}", b),
            Reply::ValueU8(v) => format!("{}", v),
            Reply::ValueU32(v) => format!("{}", v),
        };
        Response::build()
            .sized_body(body.len(), Cursor::new(body))
            .status(Status::Ok)
            .ok()
    }
}

pub struct WebErr(ControllerErr);

impl<'r, 'o: 'r> Responder<'r, 'o> for WebErr {
    fn respond_to(self, _: &'r Request<'_>) -> response::Result<'o> {
        let status = match self.0 {
            ControllerErr::SerialPortError => Status::InternalServerError,
            ControllerErr::UnsupportedCommand => Status::NotImplemented,
        };
        let body = format!("{:?}", self.0);
        Response::build()
            .sized_body(body.len(), Cursor::new(body))
            .status(status)
            .ok()
    }
}

impl From<ControllerErr> for WebErr {
    fn from(e: ControllerErr) -> Self {
        WebErr(e)
    }
}

#[get("/<command>")]
pub fn read(command: &str) -> Result<WebReply, WebErr> {
    let mut controller = Controller::new(Path::new("/dev/ttyUSB0"))?;
    let comm = get_command(command, SubCommand::Status)?;
    Ok(WebReply(controller.read(&comm)?))
}

#[put("/<command>?<state>")]
pub fn write(command: &str, state: &str) -> Result<(), WebErr>  {
    let mut controller = Controller::new(Path::new("/dev/ttyUSB0"))?;
    let sc = get_subcommand(state)?;
    if let SubCommand::Status = sc {
        return Err(WebErr(ControllerErr::UnsupportedCommand));
    }
    let write = get_command(command, sc)?;
    controller.write(&write)?;
    Ok(())
}

#[launch]
fn rocket() -> _ {
    let mut config = Config::release_default();
    config.address = IpAddr::from_str("0.0.0.0").unwrap();
    config.port = 43880;

    rocket::custom(config).mount("/", routes![read, write])
}

fn get_subcommand(state: &str) -> Result<SubCommand, WebErr> {
    match state {
        "up" => Ok(SubCommand::Up),
        "down" => Ok(SubCommand::Down),
        "status" => Ok(SubCommand::Status),
        _ => Err(WebErr(ControllerErr::UnsupportedCommand)),
    }
}

fn get_command(command: &str, state: SubCommand) -> Result<Command, WebErr> {
    match command {
        "power" => Ok(Command::Power(state)),
        "eco" => Ok(Command::Eco(state)),
        "source" => Ok(Command::Source(state)),
        "brightness" => Ok(Command::Brightness(state)),
        "volume" => Ok(Command::Volume(state)),
        "mute" => Ok(Command::Mute(state)),
        "lamp_time" => Ok(Command::Mute(state)),
        _ => Err(WebErr(ControllerErr::UnsupportedCommand)),
    }
}