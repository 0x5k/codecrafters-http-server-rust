use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
#[allow(unused_imports)]

fn main() {
    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();
    println!("Server listening on 127.0.0.1:4221");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                thread::spawn(|| {
                    println!("Accepted new connection");
                    if let Err(e) = handle_connection(stream) {
                        println!("Failed to handle connection: {}", e);
                    }
                });
            }
            Err(e) => {
                println!("Error accepting connection: {}", e);
            }
        }
    }
}

fn handle_connection(mut stream: TcpStream) -> std::io::Result<()> {
    let mut buffer = [0; 1024]; // 1KB buffer, adjust size as needed
    let bytes_read = stream.read(&mut buffer)?;

    // If no bytes are read, it might be an empty request or closed connection
    if bytes_read == 0 {
        println!("Received empty request or connection closed.");
        return Ok(()); // Nothing more to do
    }

    let request_str = String::from_utf8_lossy(&buffer[..bytes_read]);

    println!("Raw Request:\n{}", request_str);

    let request_line = request_str.lines().next().unwrap_or("");

    let parts: Vec<&str> = request_line.split_whitespace().collect();

    let mut status_line = "HTTP/1.1 404 Not Found\r\n\r\n";
    let mut headers = String::new();
    let mut response_body = "404 Not Found\r\n\r\n"; // Simple body for 404

    if parts.len() >= 2 {
        let method = parts[0];
        let path = parts[1];

        println!("Received request: {} {}", method, path); // Log the method and path

        if path == "/" {
            status_line = "HTTP/1.1 200 OK\r\n\r\n";
            response_body = ""; // No body needed for a simple 200 OK in this case
        } else if path.starts_with("/echo/") {
            let echo_str = &path["/echo/".len()..];
            status_line = "HTTP/1.1 200 OK\r\n";
            headers = format!(
                "Content-Type: text/plain\r\nContent-Length: {}\r\n\r\n",
                echo_str.len()
            );
            response_body = echo_str;
        }
    } else {
        println!("Received malformed request line: {}", request_line);
    }

    let response = format!("{}{}{}", status_line, headers, response_body);

    stream.write_all(response.as_bytes())?;
    stream.flush()?;

    println!("Response sent.");
    Ok(())
}
