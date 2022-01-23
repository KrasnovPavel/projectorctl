#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;
extern crate okapi;

use projectorctl::*;

use log::{info, warn};
use rocket::config::LogLevel;
use rocket::http::Status;
use rocket::{get, put, serde::json::Json};
use rocket::{Config, State};
use rocket_okapi::{openapi, openapi_get_routes};
use std::net::IpAddr;
use std::path::Path;
use std::str::FromStr;
use std::sync::{Arc, Mutex, MutexGuard};

type ControllerPointer = Arc<Mutex<Controller>>;

fn get_controller(
    pointer: &State<Arc<Mutex<Controller>>>,
) -> Result<MutexGuard<Controller>, ControllerErr> {
    match pointer.inner().lock() {
        Ok(r) => Ok(r),
        Err(_) => Err(ControllerErr::SerialPortError),
    }
}

#[openapi]
#[get("/<command>")]
pub fn read(
    pointer: &State<ControllerPointer>,
    command: &str,
) -> Result<Json<Reply>, (Status, Json<ControllerErr>)> {
    let controller = get_controller(pointer);
    let comm = get_command(command, &SubCommand::Status);
    if let Err(e) = comm {
        warn!("Cannot parse command {:?}", e);
        return Err((Status::NotFound, Json(e)));
    }
    let comm = comm.unwrap();
    match controller {
        Ok(mut c) => match c.read(&comm) {
            Ok(reply) => {
                info!("Get state of {:?}: {:?}", comm, reply);
                Ok(Json(reply))
            }
            Err(e) => {
                warn!("Cannot read from tty {:?}", e);
                Err((Status::InternalServerError, Json(e)))
            }
        },
        Err(e) => {
            warn!("Cannot get controller {:?}", e);
            Err((Status::InternalServerError, Json(e)))
        }
    }
}

#[openapi]
#[put("/<command>", data = "<subcommand>")]
pub fn write(
    pointer: &State<ControllerPointer>,
    command: &str,
    subcommand: Json<SubCommand>,
) -> Result<(), (Status, Json<ControllerErr>)> {
    let controller = get_controller(pointer);
    if let SubCommand::Status = subcommand.0 {
        return Err((
            Status::NotAcceptable,
            Json(ControllerErr::UnsupportedCommand),
        ));
    }
    let comm = get_command(command, &subcommand.0);
    if let Err(e) = comm {
        warn!("Cannot parse command {:?}", e);
        return Err((Status::NotFound, Json(e)));
    }
    let comm = comm.unwrap();

    match controller {
        Ok(mut c) => match c.write(&comm) {
            Ok(_) => {
                info!("Set {:?}", comm);
                return Ok(());
            }
            Err(e) => {
                warn!("Cannot write to tty {:?}", e);
                return Err((Status::InternalServerError, Json(e)));
            }
        },
        Err(e) => {
            warn!("Cannot get controller {:?}", e);
            return Err((Status::InternalServerError, Json(e)));
        }
    }
}

#[launch]
fn rocket() -> _ {
    let mut config = Config::release_default();
    config.address = IpAddr::from_str("0.0.0.0").unwrap();
    config.port = 43880;
    config.log_level = LogLevel::Normal;

    let controller = Controller::new(Path::new("/dev/ttyUSB0"));
    let pointer = Arc::new(Mutex::new(controller.expect("Controller was not created")));

    rocket::custom(config)
        .manage(pointer)
        .mount("/", openapi_get_routes![read, write])
}

fn get_command(command: &str, state: &SubCommand) -> Result<Command, ControllerErr> {
    match command {
        "power" => Ok(Command::Power(state.clone())),
        "eco" => Ok(Command::Eco(state.clone())),
        "source" => Ok(Command::Source(state.clone())),
        "brightness" => Ok(Command::Brightness(state.clone())),
        "volume" => Ok(Command::Volume(state.clone())),
        "mute" => Ok(Command::Mute(state.clone())),
        "lamp_time" => Ok(Command::Mute(state.clone())),
        _ => Err(ControllerErr::UnsupportedCommand),
    }
}
