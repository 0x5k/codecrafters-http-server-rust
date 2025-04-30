use crate::utils::headers::parse_headers;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read};
use std::net::TcpStream;

pub struct Request {
    method: String,
    path: String,
    headers: HashMap<String, String>,
    body: Option<Vec<u8>>,
}

impl Request {
    pub fn from_reader(reader: &mut BufReader<&TcpStream>) -> std::io::Result<Option<Self>> {
        // Read request line
        let mut request_line = String::new();
        if reader.read_line(&mut request_line)? == 0 {
            return Ok(None); // Client closed connection
        }

        let trimmed_line = request_line.trim();
        if trimmed_line.is_empty() {
            return Ok(None);
        }

        let parts: Vec<&str> = trimmed_line.split_whitespace().collect();
        if parts.len() < 2 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Malformed request line",
            ));
        }

        let method = parts[0].to_string();
        let path = parts[1].to_string();
        let headers = parse_headers(reader)?;

        // Read body if needed
        let mut body = None;
        if method == "POST" && path.starts_with("/files/") {
            if let Some(content_length) = headers.get("content-length") {
                if let Ok(length) = content_length.parse::<usize>() {
                    if length > 0 {
                        let mut buffer = vec![0; length];
                        reader.read_exact(&mut buffer)?;
                        body = Some(buffer);
                    }
                }
            }
        }

        Ok(Some(Request {
            method,
            path,
            headers,
            body,
        }))
    }

    pub fn method(&self) -> &str {
        &self.method
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn headers(&self) -> &HashMap<String, String> {
        &self.headers
    }

    pub fn body(&self) -> Option<&Vec<u8>> {
        self.body.as_ref()
    }

    pub fn should_close(&self) -> bool {
        self.headers
            .get("connection")
            .map_or(false, |h| h.eq_ignore_ascii_case("close"))
    }
}
