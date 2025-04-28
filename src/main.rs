use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
fn main() {
    // Bind the listener to the local address and port
    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();
    println!("Server listening on 127.0.0.1:4221");

    // Accept incoming connections in a loop
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                // Spawn a new thread to handle each connection
                // allows the server to handle multiple clients concurrently
                thread::spawn(|| {
                    println!("Accepted new connection");
                    if let Err(e) = handle_connection(stream) {
                        println!("Failed to handle connection: {}", e);
                    }
                });
            }
            Err(e) => {
                // Print an error if accepting a connection fails
                println!("Error accepting connection: {}", e);
            }
        }
    }
}

// Function to handle a single client connection
fn handle_connection(mut stream: TcpStream) -> std::io::Result<()> {
    // Create a buffer to store the incoming request data
    let mut buffer = [0; 1024]; // 1KB buffer, adjust size as needed
                                // Read data from the stream into the buffer
    let bytes_read = stream.read(&mut buffer)?;

    // If no bytes are read, it might be an empty request or closed connection
    if bytes_read == 0 {
        println!("Received empty request or connection closed.");
        return Ok(()); // Nothing more to do
    }

    // Convert the read bytes into a UTF-8 string slice
    // We only use the part of the buffer that was actually filled (`..bytes_read`)
    let request_str = String::from_utf8_lossy(&buffer[..bytes_read]);

    // Print the raw request for debugging
    println!("Raw Request:\n{}", request_str);

    // Find the first line (request line) of the HTTP request
    let request_line = request_str.lines().next().unwrap_or("");

    // Split the request line by spaces (e.g., "GET /path HTTP/1.1")
    let parts: Vec<&str> = request_line.split_whitespace().collect();

    // Default response is 404 Not Found
    let mut status_line = "HTTP/1.1 404 Not Found\r\n\r\n";
    let mut response_body = "404 Not Found"; // Simple body for 404

    // Check if the request line has at least 3 parts (Method, Path, Protocol)
    if parts.len() >= 2 {
        let method = parts[0];
        let path = parts[1];

        println!("Received request: {} {}", method, path); // Log the method and path

        // Check if the path is the root path "/"
        if path == "/" {
            // If it's the root path, set the response to 200 OK
            status_line = "HTTP/1.1 200 OK\r\n\r\n";
            response_body = ""; // No body needed for a simple 200 OK in this case
        }
        // Add more `else if` blocks here to handle other paths like /echo/{str} or /user-agent
        // else if path.starts_with("/echo/") { ... }
        // else if path == "/user-agent" { ... }
    } else {
        println!("Received malformed request line: {}", request_line);
        // Keep the default 404 response for malformed requests
    }

    // Combine the status line and the body (if any) for the final response
    // Note: For simplicity, we are not including Content-Length or Content-Type headers here.
    // A more robust server would need these.
    let response = format!("{}{}", status_line, response_body);

    // Write the response back to the client stream
    stream.write_all(response.as_bytes())?;
    // Flush the stream to ensure all data is sent immediately
    stream.flush()?;

    println!("Response sent.");
    Ok(()) // Indicate success
}
