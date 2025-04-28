use std::io::{Read, Write};
#[allow(unused_imports)]
use std::net::{TcpListener, TcpStream};

fn main() {
    let listener = TcpListener::bind("localhost:4221").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(mut _stream) => {
                println!("accepted new connection");
                _stream.write(b"HTTP/1.1 200 OK\r\n\r\n").unwrap();
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}
