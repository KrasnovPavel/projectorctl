use projectorctl::*;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
struct Cli {
    #[structopt(short, parse(from_os_str))]
    path: PathBuf,
    #[structopt(subcommand)]
    command: Command,
}

fn main() {
    let args: Cli = Cli::from_args();
    let mut c = Controller::new(args.path.as_path()).unwrap();
    if args.command.is_readable() {
        println!(
            "{:#?}",
            c.read(&args.command)
                .expect("Read from serial port failed!")
        );
    } else {
        c.write(&args.command).unwrap();
    }
}
