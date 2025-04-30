use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::thread;
// Removed unused import: use std::time::Duration;

#[allow(unused_imports)]
fn main() {
    // --- Argument Parsing ---
    let args: Vec<String> = env::args().collect();
    let mut directory = String::from("."); // Default to current directory

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

    // Bind to 0.0.0.0 to accept connections from any network interface
    let bind_address = "0.0.0.0:4221";
    let listener = match TcpListener::bind(bind_address) {
        Ok(listener) => listener,
        Err(e) => {
            eprintln!("Failed to bind to {}: {}", bind_address, e);
            return;
        }
    };
    println!("Server listening on {}", bind_address);

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let dir_clone = directory.clone();
                // Note: Setting timeouts might require careful handling with persistent connections
                // stream.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
                // stream.set_write_timeout(Some(Duration::from_secs(10))).unwrap();

                thread::spawn(move || {
                    let peer_addr = stream
                        .peer_addr()
                        .map_or_else(|_| "unknown".to_string(), |addr| addr.to_string());
                    println!("Accepted new connection from: {}", peer_addr);

                    // Pass the directory to the handler
                    // Pass the stream itself now, handle_connection will manage reading/writing
                    if let Err(e) = handle_connection(stream, &dir_clone) {
                        // Log errors that cause the connection handler to terminate
                        match e.kind() {
                            // Ignore BrokenPipe and ConnectionReset errors as they often mean the client disconnected normally
                            std::io::ErrorKind::BrokenPipe
                            | std::io::ErrorKind::ConnectionReset
                            | std::io::ErrorKind::UnexpectedEof => {
                                // Also ignore UnexpectedEof here
                                println!("Client {} disconnected.", peer_addr);
                            }
                            // Log other I/O errors
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
}

// Function to parse headers into a HashMap
// Takes BufReader by mutable reference, consumes header lines
// Note: Takes BufReader<(&TcpStream)> to work with the reader created in handle_connection
fn parse_headers(reader: &mut BufReader<&TcpStream>) -> std::io::Result<HashMap<String, String>> {
    let mut headers = HashMap::new();
    let mut header_line = String::new();

    loop {
        // Read line by line until an empty line (\r\n) is encountered
        let bytes_read = reader.read_line(&mut header_line)?;
        if bytes_read == 0 {
            // Connection closed unexpectedly during header reading
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "Connection closed while reading headers",
            ));
        }

        // Trim whitespace (including \r\n)
        let trimmed_line = header_line.trim();
        if trimmed_line.is_empty() {
            // Empty line signifies end of headers
            break;
        }
        // Split header name and value
        if let Some((name, value)) = trimmed_line.split_once(": ") {
            headers.insert(name.to_lowercase(), value.trim().to_string()); // Trim header value too
        } else {
            // Malformed header line, could return error or ignore
            println!("Warning: Malformed header line ignored: {}", trimmed_line);
        }
        header_line.clear(); // Clear string for the next line
    }
    Ok(headers)
}

// Handles multiple requests on a single connection
// Takes ownership of the stream
fn handle_connection(mut stream: TcpStream, directory: &str) -> std::io::Result<()> {
    // Loop to handle multiple requests on the same connection
    loop {
        // Create a BufReader for *each request* within the loop.
        // This limits the scope of the immutable borrow of `stream`.
        let mut reader = BufReader::new(&stream); // Immutable borrow starts here

        // --- Read Request Line ---
        let mut request_line = String::new();
        match reader.read_line(&mut request_line) {
            Ok(0) => {
                // Client closed connection gracefully.
                break; // Exit loop, close connection
            }
            Ok(_) => {
                // Continue processing request
                // Avoid printing empty lines if client sends extra CRLFs
                if !request_line.trim().is_empty() {
                    println!("Request Line: {}", request_line.trim());
                }
            }
            Err(e) => {
                // Error reading request line (could be timeout, network issue, etc.)
                return Err(e); // Propagate I/O error up
            }
        }

        let trimmed_request_line = request_line.trim();
        if trimmed_request_line.is_empty() {
            // Received an empty line (potentially keep-alive probe or end of pipelined requests)
            // Continue listening without processing it as a full request.
            request_line.clear();
            continue; // Go to next loop iteration
        }

        // --- Parse Request Line ---
        let parts: Vec<&str> = trimmed_request_line.split_whitespace().collect();
        if parts.len() < 2 {
            // Technically needs 3 (Method, Path, Version) but be lenient
            println!(
                "Malformed request line ({} parts): {}",
                parts.len(),
                trimmed_request_line
            );
            // Respond with 400 and close immediately
            stream.write_all(b"HTTP/1.1 400 Bad Request\r\nConnection: close\r\n\r\n")?;
            stream.flush()?;
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Malformed request line",
            )); // Return error to close handler
        }
        let method = parts[0];
        let path = parts[1];

        // --- Parse Headers ---
        // Pass the reader mutably to consume header lines
        let headers_map = match parse_headers(&mut reader) {
            Ok(headers) => headers,
            Err(e) => {
                println!("Error parsing headers: {}", e);
                stream.write_all(b"HTTP/1.1 400 Bad Request\r\nConnection: close\r\n\r\n")?;
                stream.flush()?;
                return Err(e); // Propagate error
            }
        };
        println!("Headers: {:?}", headers_map);

        // --- Determine if connection should close ---
        let mut should_close = headers_map
            .get("connection")
            .map_or(false, |h| h.eq_ignore_ascii_case("close"));

        // --- Read Body (if necessary, e.g., for POST) ---
        let mut request_body_content: Option<Vec<u8>> = None;
        let mut body_read_error = false; // Flag to track errors during body reading

        if method == "POST" && path.starts_with("/files/") {
            if let Some(content_length_str) = headers_map.get("content-length") {
                if let Ok(content_length) = content_length_str.parse::<usize>() {
                    if content_length > 0 {
                        // Only read if Content-Length is positive
                        let mut body_buffer = vec![0; content_length];
                        // Read the body using the *same reader*
                        if reader.read_exact(&mut body_buffer).is_ok() {
                            request_body_content = Some(body_buffer);
                        } else {
                            println!(
                                "Failed to read request body fully (expected {} bytes)",
                                content_length
                            );
                            body_read_error = true; // Mark error
                            should_close = true; // Close connection on incomplete body
                        }
                    } else {
                        // Content-Length is 0, treat as empty body
                        request_body_content = Some(Vec::new());
                    }
                } else {
                    println!("Invalid Content-Length header: {}", content_length_str);
                    body_read_error = true; // Mark error
                    should_close = true; // Close on bad header
                }
            } else {
                println!("Missing Content-Length header for POST");
                body_read_error = true; // Mark error
                should_close = true; // 411 Length Required implies close
            }
        }

        // --- End of Reading Phase ---
        // The immutable borrow of `stream` by `reader` ends here.

        // --- Routing Logic & Response Preparation ---
        let mut status_line = "HTTP/1.1 404 Not Found\r\n";
        let mut response_headers_vec: Vec<String> = Vec::new();
        let mut response_body: Option<Vec<u8>> = None;

        // Check for errors detected during body reading first
        if body_read_error {
            if headers_map.get("content-length").is_none() {
                status_line = "HTTP/1.1 411 Length Required\r\n";
            } else {
                // Could be invalid Content-Length or read_exact failure
                status_line = "HTTP/1.1 400 Bad Request\r\n";
            }
            // should_close is already true
            response_body = Some(status_line.as_bytes().to_vec()); // Optionally send status as body for errors
        } else {
            // Proceed with normal routing if no body read error occurred
            match (method, path) {
                ("GET", "/") => {
                    status_line = "HTTP/1.1 200 OK\r\n";
                }
                ("GET", "/files/") => {
                    // Directory listing as JSON with metadata
                    match fs::read_dir(directory) {
                        Ok(entries) => {
                            let mut items = Vec::new();
                            for entry in entries.flatten() {
                                if let (Ok(metadata), Some(name_str)) =
                                    (entry.metadata(), entry.file_name().to_str())
                                {
                                    let type_str = if metadata.is_dir() {
                                        "dir"
                                    } else if metadata.is_file() {
                                        "file"
                                    } else {
                                        "other"
                                    };

                                    items.push(format!(
                                        "{{\"name\":\"{}\",\"type\":\"{}\",\"size\":{},\"modified\":{}}}",
                                        name_str,
                                        type_str,
                                        metadata.len(),
                                        metadata.modified()
                                            .unwrap_or_else(|_| std::time::SystemTime::UNIX_EPOCH)
                                            .duration_since(std::time::UNIX_EPOCH)
                                            .unwrap_or_default()
                                            .as_secs()
                                    ));
                                }
                            }
                            let body = format!("[{}]", items.join(","));
                            status_line = "HTTP/1.1 200 OK\r\n";
                            response_headers_vec.push("Content-Type: application/json".to_string());
                            response_headers_vec.push(format!("Content-Length: {}", body.len()));
                            response_body = Some(body.into_bytes());
                        }
                        Err(e) => {
                            status_line = "HTTP/1.1 500 Internal Server Error\r\n";
                            response_body =
                                Some(format!("Failed to read directory: {}", e).into_bytes());
                        }
                    }
                }
                ("GET", p) if p.starts_with("/echo/") => {
                    let echo_str = &p["/echo/".len()..];
                    status_line = "HTTP/1.1 200 OK\r\n";
                    response_headers_vec.push(format!("Content-Type: text/plain"));
                    response_headers_vec.push(format!("Content-Length: {}", echo_str.len()));
                    response_body = Some(echo_str.as_bytes().to_vec());
                }
                ("GET", "/user-agent") => {
                    if let Some(user_agent) = headers_map.get("user-agent") {
                        status_line = "HTTP/1.1 200 OK\r\n";
                        response_headers_vec.push(format!("Content-Type: text/plain"));
                        response_headers_vec.push(format!("Content-Length: {}", user_agent.len()));
                        response_body = Some(user_agent.as_bytes().to_vec());
                    } else {
                        // Respond with 200 OK and empty body if User-Agent is missing
                        status_line = "HTTP/1.1 200 OK\r\n";
                        response_headers_vec.push(format!("Content-Type: text/plain"));
                        response_headers_vec.push(format!("Content-Length: 0"));
                        response_body = Some(Vec::new());
                    }
                }
                ("GET", p) if p.starts_with("/files/") => {
                    let filename = &p["/files/".len()..];
                    // Basic path validation: prevent directory traversal
                    if filename.contains("..") || filename.starts_with('/') {
                        status_line = "HTTP/1.1 400 Bad Request\r\n";
                        should_close = true;
                    } else {
                        let file_path = Path::new(directory).join(filename);
                        println!("Attempting to serve file: {:?}", file_path);

                        match fs::read(&file_path) {
                            Ok(contents) => {
                                status_line = "HTTP/1.1 200 OK\r\n";
                                response_headers_vec
                                    .push(format!("Content-Type: application/octet-stream"));
                                response_headers_vec
                                    .push(format!("Content-Length: {}", contents.len()));
                                response_body = Some(contents);
                                println!("Successfully read file: {:?}", file_path);
                            }
                            Err(e) => {
                                println!("Failed to read file {:?}: {}", file_path, e);
                                // Keep default 404 Not Found
                                status_line = "HTTP/1.1 404 Not Found\r\n";
                            }
                        }
                    }
                }
                ("POST", p) if p.starts_with("/files/") => {
                    let filename = &p["/files/".len()..];
                    // Basic path validation: prevent directory traversal
                    if filename.contains("..") || filename.starts_with('/') {
                        status_line = "HTTP/1.1 400 Bad Request\r\n";
                        should_close = true;
                    } else if let Some(body_data) = request_body_content {
                        // Body was read successfully
                        let file_path = Path::new(directory).join(filename);
                        println!("Attempting to write file: {:?}", file_path);
                        match fs::write(&file_path, &body_data) {
                            Ok(_) => {
                                status_line = "HTTP/1.1 201 Created\r\n";
                                println!("Successfully created file: {:?}", file_path);
                                // No body for 201 response
                                response_body = None;
                            }
                            Err(e) => {
                                println!("Failed to write file {:?}: {}", file_path, e);
                                status_line = "HTTP/1.1 500 Internal Server Error\r\n";
                                should_close = true;
                                response_body =
                                    Some(format!("Failed to write file: {}", e).into_bytes());
                            }
                        }
                    } else {
                        // This case should be caught by body_read_error check above
                        println!("Error: Reached POST /files/ handler without valid body data and no prior error.");
                        status_line = "HTTP/1.1 500 Internal Server Error\r\n";
                        should_close = true;
                    }
                }
                _ => {
                    println!("Path not handled: {} {}", method, path);
                    // Keep default 404 Not Found
                    status_line = "HTTP/1.1 404 Not Found\r\n";
                }
            }
        }

        // --- Write Response ---
        // Now we can get a mutable borrow of stream.
        if should_close {
            // Ensure Connection: close is present if we decided to close
            if !response_headers_vec
                .iter()
                .any(|h| h.eq_ignore_ascii_case("Connection: close"))
            {
                response_headers_vec.push("Connection: close".to_string());
            }
        }

        // Write status line
        stream.write_all(status_line.as_bytes())?;

        // Write headers
        for header in &response_headers_vec {
            // Borrow vec instead of consuming
            stream.write_all(header.as_bytes())?;
            stream.write_all(b"\r\n")?;
        }
        stream.write_all(b"\r\n")?; // Final CRLF

        // Write body
        if let Some(body) = &response_body {
            // Borrow body instead of consuming
            stream.write_all(body)?;
        }

        stream.flush()?; // Ensure response is sent

        println!("Response sent. Connection close: {}", should_close);

        if should_close {
            break; // Exit loop
        }
    } // End of request handling loop

    Ok(())
}
