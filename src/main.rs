#![feature(async_closure)]
#![feature(fn_traits)]
#![cfg_attr(feature = "flame_it", feature(proc_macro_hygiene))]
#[cfg(feature = "flame_it")]
extern crate flame;
#[cfg(feature = "flame_it")]
#[macro_use] extern crate flamer;

mod utils;
mod server;
mod pact;
mod cli;
mod web;
mod pacts;

use crate::server::{PactServer};

#[tokio::main]
async fn main() {
    let (pact_files_folder, port) = match cli::get_commandline_args() {
        Ok((pact_file, port)) => (pact_file, port),
        Err(e) => {
            println!("Error: {e}");
            return;
        }
    };
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let mut pact_server = PactServer::with_http_server(
        &pact_files_folder,
        None,
        Some(port))
        .expect("Error creating pact server");
    pact_server.start_blocking().await.expect("Error starting pact server");
}


