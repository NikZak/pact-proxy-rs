use clap::{arg, command, value_parser};
use std::error::Error;
use std::net::TcpListener;
use std::path::PathBuf;

fn unwrap_commandline_args(
    matches: &clap::ArgMatches,
) -> Result<(PathBuf, String), Box<dyn Error>> {
    let pact_file = matches
        .get_one::<PathBuf>("pact_files_folder")
        .ok_or("Can't open pact_file")?
        .to_path_buf();
    let port = parse_port(matches)?;
    Ok((pact_file, port))
}

fn parse_port(matches: &clap::ArgMatches) -> Result<String, Box<dyn Error>> {
    let default_port = get_rand_port().to_string();
    let port = matches
        .get_one::<String>("port")
        .unwrap_or(&default_port)
        .to_string();
    Ok(port)
}

pub fn get_commandline_args() -> Result<(PathBuf, String), Box<dyn Error>> {
    let matches = command!()
        .arg(
            arg!(-f --pact_files_folder <FILE> "The pact file to load")
                .required(true)
                .value_parser(value_parser!(PathBuf)),
        )
        .arg(arg!(-p --port <PORT> "The port to run the mock service on").required(false))
        .get_matches();
    unwrap_commandline_args(&matches)
}

pub fn get_rand_port() -> u16 {
    let mut port = rand::random::<u16>() % 1000 + 10000;
    while !port_is_available(port) {
        port = rand::random::<u16>() % 1000 + 10000
    }
    port
}

fn port_is_available(port: u16) -> bool {
    TcpListener::bind(("127.0.0.1", port)).is_ok()
}
