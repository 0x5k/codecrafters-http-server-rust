use crate::server::Server;
use std::env;

mod server;
mod utils;

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut directory = String::from(".");

    if let Some(index) = args.iter().position(|arg| arg == "--directory") {
        if let Some(dir) = args.get(index + 1) {
            directory = dir.clone();
            println!("Serving files from directory: {}", directory);
        } else {
            eprintln!("Error: --directory flag requires an argument.");
            return;
        }
    }

    let server = Server::new("0.0.0.0:4221".to_string(), directory);
    if let Err(e) = server.run() {
        eprintln!("Server error: {}", e);
    }
}
