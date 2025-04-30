use crate::server::handler::handle_connection;
use std::io;
use std::net::{TcpListener, TcpStream};
use std::thread;

pub struct Server {
    address: String,
    directory: String,
}

impl Server {
    pub fn new(address: String, directory: String) -> Self {
        Self { address, directory }
    }

    pub fn run(&self) -> io::Result<()> {
        let listener = TcpListener::bind(&self.address)?;
        println!("Server listening on {}", self.address);

        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let dir_clone = self.directory.clone();
                    thread::spawn(move || {
                        let peer_addr = stream
                            .peer_addr()
                            .map_or_else(|_| "unknown".to_string(), |addr| addr.to_string());
                        println!("Accepted new connection from: {}", peer_addr);

                        if let Err(e) = handle_connection(stream, &dir_clone) {
                            match e.kind() {
                                io::ErrorKind::BrokenPipe
                                | io::ErrorKind::ConnectionReset
                                | io::ErrorKind::UnexpectedEof => {
                                    println!("Client {} disconnected.", peer_addr);
                                }
                                _ => println!("Connection handler error for {}: {}", peer_addr, e),
                            }
                        }
                        println!("Connection from {} closed.", peer_addr);
                    });
                }
                Err(e) => {
                    println!("Error accepting connection: {}", e);
                }
            }
        }
        Ok(())
    }
}

pub mod handler;
pub mod request;
pub mod response;
pub mod router;
