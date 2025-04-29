use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{BufRead, BufReader, Read, Write}; // Added BufRead, BufReader
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::thread;

#[allow(unused_imports)]
fn main() {
    // --- Argument Parsing ---
    let args: Vec<String> = env::args().collect();
    let mut directory = String::from("."); // Default to current directory if not specified

    if let Some(index) = args.iter().position(|arg| arg == "--directory") {
        if let Some(dir) = args.get(index + 1) {
            directory = dir.clone();
            println!("Serving files from directory: {}", directory);
        } else {
            eprintln!("Error: --directory flag requires an argument.");
            return;
        }
    }
    // --- End Argument Parsing ---

    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();
    println!("Server listening on 127.0.0.1:4221");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let dir_clone = directory.clone();
                thread::spawn(move || {
                    println!("Accepted new connection");
                    if let Err(e) = handle_connection(stream, &dir_clone) {
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

// Function to parse headers into a HashMap
fn parse_headers(reader: &mut BufReader<&TcpStream>) -> HashMap<String, String> {
    let mut headers = HashMap::new();
    let mut header_line = String::new();

    loop {
        // Read line by line until an empty line (\r\n) is encountered
        match reader.read_line(&mut header_line) {
            Ok(0) => break, // Connection closed unexpectedly
            Ok(_) => {
                // Trim whitespace (including \r\n)
                let trimmed_line = header_line.trim();
                if trimmed_line.is_empty() {
                    // Empty line signifies end of headers
                    break;
                }
                // Split header name and value
                if let Some((name, value)) = trimmed_line.split_once(": ") {
                    headers.insert(name.to_lowercase(), value.to_string());
                }
                header_line.clear(); // Clear string for the next line
            }
            Err(_) => break, // Error reading line
        }
    }
    headers
}

fn handle_connection(mut stream: TcpStream, directory: &str) -> std::io::Result<()> {
    // Use BufReader for more efficient reading, especially line-by-line for headers
    let mut reader = BufReader::new(&stream);

    // Read the request line
    let mut request_line = String::new();
    if reader.read_line(&mut request_line)? == 0 {
        println!("Received empty request or connection closed.");
        return Ok(()); // Client closed connection
    }
    println!("Request Line: {}", request_line.trim());

    // Parse the request line
    let parts: Vec<&str> = request_line.trim().split_whitespace().collect();
    if parts.len() < 3 {
        println!("Malformed request line: {}", request_line.trim());
        // Send 400 Bad Request for malformed request line
        stream.write_all(b"HTTP/1.1 400 Bad Request\r\n\r\n")?;
        stream.flush()?;
        return Ok(());
    }
    let method = parts[0];
    let path = parts[1];
    // let http_version = parts[2]; // Not used yet

    // Parse headers
    let headers_map = parse_headers(&mut reader);
    println!("Headers: {:?}", headers_map); // Log parsed headers

    // Default response: 404 Not Found
    let mut status_line = "HTTP/1.1 404 Not Found\r\n";
    let mut response_headers = "\r\n".to_string(); // Default: just CRLF ending headers
    let mut response_body: Option<Vec<u8>> = None;

    // --- Routing Logic ---
    match (method, path) {
        ("GET", "/") => {
            status_line = "HTTP/1.1 200 OK\r\n";
            // response_headers remains default (just CRLF)
        }
        ("GET", p) if p.starts_with("/echo/") => {
            let echo_str = &p["/echo/".len()..];
            status_line = "HTTP/1.1 200 OK\r\n";
            response_headers = format!(
                "Content-Type: text/plain\r\nContent-Length: {}\r\n\r\n",
                echo_str.len()
            );
            response_body = Some(echo_str.as_bytes().to_vec());
        }
        ("GET", "/user-agent") => {
            if let Some(user_agent) = headers_map.get("user-agent") {
                status_line = "HTTP/1.1 200 OK\r\n";
                response_headers = format!(
                    "Content-Type: text/plain\r\nContent-Length: {}\r\n\r\n",
                    user_agent.len()
                );
                response_body = Some(user_agent.as_bytes().to_vec());
            } else {
                // Should technically not happen if User-Agent is always sent, but handle defensively
                status_line = "HTTP/1.1 400 Bad Request\r\n"; // Or maybe 500?
                response_headers = "Content-Type: text/plain\r\n\r\n".to_string();
                response_body = Some(b"User-Agent header not found".to_vec());
            }
        }
        ("GET", p) if p.starts_with("/files/") => {
            let filename = &p["/files/".len()..];
            let file_path = Path::new(directory).join(filename);
            println!("Attempting to serve file: {:?}", file_path);

            match fs::read(&file_path) {
                Ok(contents) => {
                    status_line = "HTTP/1.1 200 OK\r\n";
                    response_headers = format!(
                        "Content-Type: application/octet-stream\r\nContent-Length: {}\r\n\r\n",
                        contents.len()
                    );
                    response_body = Some(contents);
                    println!("Successfully read file: {:?}", file_path);
                }
                Err(e) => {
                    println!("Failed to read file {:?}: {}", file_path, e);
                    // Keep default 404 status_line and response_headers
                }
            }
        }
        // --- POST /files handling ---
        ("POST", p) if p.starts_with("/files/") => {
            let filename = &p["/files/".len()..];
            let file_path = Path::new(directory).join(filename);
            println!("Attempting to create file: {:?}", file_path);

            // Get Content-Length from headers
            if let Some(content_length_str) = headers_map.get("content-length") {
                if let Ok(content_length) = content_length_str.parse::<usize>() {
                    // Read the exact number of bytes for the body
                    let mut body_buffer = vec![0; content_length];
                    if reader.read_exact(&mut body_buffer).is_ok() {
                        // Write the body content to the file
                        match fs::write(&file_path, &body_buffer) {
                            Ok(_) => {
                                status_line = "HTTP/1.1 201 Created\r\n";
                                response_headers = "\r\n".to_string(); // No extra headers needed
                                response_body = None; // No body for 201
                                println!("Successfully created file: {:?}", file_path);
                            }
                            Err(e) => {
                                // Error writing file
                                println!("Failed to write file {:?}: {}", file_path, e);
                                status_line = "HTTP/1.1 500 Internal Server Error\r\n";
                                response_headers = "\r\n".to_string();
                                response_body = None;
                            }
                        }
                    } else {
                        // Error reading body from stream
                        println!("Failed to read request body");
                        status_line = "HTTP/1.1 400 Bad Request\r\n";
                        response_headers = "\r\n".to_string();
                        response_body = None;
                    }
                } else {
                    // Invalid Content-Length value
                    println!("Invalid Content-Length header");
                    status_line = "HTTP/1.1 400 Bad Request\r\n";
                    response_headers = "\r\n".to_string();
                    response_body = None;
                }
            } else {
                // Content-Length header missing
                println!("Missing Content-Length header for POST");
                status_line = "HTTP/1.1 411 Length Required\r\n"; // Standard response for missing Content-Length on POST
                response_headers = "\r\n".to_string();
                response_body = None;
            }
        }
        // --- End POST /files handling ---
        _ => {
            // Path not matched, keep default 404
            println!("Path not handled: {} {}", method, path);
        }
    }

    // --- Write response ---
    stream.write_all(status_line.as_bytes())?;
    stream.write_all(response_headers.as_bytes())?;
    if let Some(body) = response_body {
        stream.write_all(&body)?;
    }

    stream.flush()?; // Ensure all data is sent
    println!("Response sent.");
    Ok(())
}
