use crate::server::request::Request;
use crate::server::response::Response;
use crate::server::router::Router;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;

pub fn handle_connection(mut stream: TcpStream, directory: &str) -> std::io::Result<()> {
    let mut router = Router::new(directory);

    loop {
        let mut reader = BufReader::new(&stream);
        let request = match Request::from_reader(&mut reader) {
            Ok(Some(req)) => req,
            Ok(None) => break, // Client closed connection
            Err(e) => {
                println!("Error reading request: {}", e);
                let response = Response::bad_request();
                response.send(&mut stream)?;
                return Err(e);
            }
        };

        let response = router.handle_request(&request);
        response.send(&mut stream)?;

        if request.should_close() {
            break;
        }
    }
    Ok(())
}
