use std::env;
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::thread;

#[allow(unused_imports)]
fn main() {
    // --- Argument Parsing ---
    let args: Vec<String> = env::args().collect();
    let mut directory = String::from("."); // Default to current directory if not specified

    // Simple argument parsing for --directory
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
                // Clone the directory path for the thread
                let dir_clone = directory.clone();
                thread::spawn(move || {
                    println!("Accepted new connection");
                    // Pass the directory to the handler
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

// Modified to accept the directory path
fn handle_connection(mut stream: TcpStream, directory: &str) -> std::io::Result<()> {
    let mut buffer = [0; 1024]; // 1KB buffer
    let bytes_read = stream.read(&mut buffer)?;

    if bytes_read == 0 {
        println!("Received empty request or connection closed.");
        return Ok(());
    }

    let request_str = String::from_utf8_lossy(&buffer[..bytes_read]);
    println!("Raw Request:\n{}", request_str);

    let request_line = request_str.lines().next().unwrap_or("");
    let parts: Vec<&str> = request_line.split_whitespace().collect();

    // Default to 404 Not Found
    let mut status_line = "HTTP/1.1 404 Not Found\r\n";
    let mut headers = String::new();
    let mut response_body: Option<Vec<u8>> = None; // Use Option<Vec<u8>> for body

    if parts.len() >= 3 {
        // Need Method, Path, Version
        let method = parts[0];
        let path = parts[1];
        // let version = parts[2]; // Not strictly needed for this logic yet

        println!("Received request: {} {}", method, path);

        if path == "/" {
            status_line = "HTTP/1.1 200 OK\r\n";
            headers = "\r\n".to_string(); // Just CRLF for empty headers
        } else if path.starts_with("/echo/") {
            let echo_str = &path["/echo/".len()..];
            status_line = "HTTP/1.1 200 OK\r\n";
            headers = format!(
                "Content-Type: text/plain\r\nContent-Length: {}\r\n\r\n",
                echo_str.len()
            );
            response_body = Some(echo_str.as_bytes().to_vec());
        } else if path == "/user-agent" {
            let user_agent = request_str
                .lines()
                .find(|line| line.to_lowercase().starts_with("user-agent:"))
                .map(|line| line[line.find(':').unwrap_or(0) + 1..].trim())
                .unwrap_or("");

            status_line = "HTTP/1.1 200 OK\r\n";
            headers = format!(
                "Content-Type: text/plain\r\nContent-Length: {}\r\n\r\n",
                user_agent.len()
            );
            response_body = Some(user_agent.as_bytes().to_vec());
        // --- New /files endpoint handling ---
        } else if method == "GET" && path.starts_with("/files/") {
            let filename = &path["/files/".len()..];
            let file_path = Path::new(directory).join(filename);

            println!("Attempting to serve file: {:?}", file_path); // Log file path

            match fs::read(&file_path) {
                Ok(contents) => {
                    status_line = "HTTP/1.1 200 OK\r\n";
                    headers = format!(
                        "Content-Type: application/octet-stream\r\nContent-Length: {}\r\n\r\n",
                        contents.len()
                    );
                    response_body = Some(contents);
                    println!("Successfully read file: {:?}", file_path);
                }
                Err(e) => {
                    // File not found or other read error, keep the default 404
                    println!("Failed to read file {:?}: {}", file_path, e);
                    status_line = "HTTP/1.1 404 Not Found\r\n";
                    headers = "\r\n".to_string(); // Just CRLF for empty headers
                    response_body = None;
                }
            }
        // --- End /files endpoint handling ---
        } else {
            // Keep default 404 for other paths
            headers = "\r\n".to_string();
        }
    } else {
        println!("Received malformed request line: {}", request_line);
        // Use a more specific error for malformed requests
        status_line = "HTTP/1.1 400 Bad Request\r\n";
        headers = "\r\n".to_string();
    }

    // --- Write response ---
    // Write status line and headers (as bytes)
    stream.write_all(status_line.as_bytes())?;
    stream.write_all(headers.as_bytes())?;

    // Write body if it exists
    if let Some(body) = response_body {
        stream.write_all(&body)?;
    } else if status_line.contains("404") {
        // Optionally write a small body for 404 if desired, but not required by spec here
        // stream.write_all(b"Not Found")?;
    }

    stream.flush()?; // Ensure all data is sent

    println!("Response sent.");
    Ok(())
}
