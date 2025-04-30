use std::fs;
use std::io::Write;
use std::path::Path;

pub struct Response {
    status_line: String,
    headers: Vec<String>,
    body: Option<Vec<u8>>,
}

impl Response {
    pub fn new(status_line: String, headers: Vec<String>, body: Option<Vec<u8>>) -> Self {
        Self {
            status_line,
            headers,
            body,
        }
    }

    pub fn ok() -> Self {
        Self::new("HTTP/1.1 200 OK\r\n".to_string(), vec![], None)
    }

    pub fn not_found() -> Self {
        Self::new("HTTP/1.1 404 Not Found\r\n".to_string(), vec![], None)
    }

    pub fn bad_request() -> Self {
        Self::new(
            "HTTP/1.1 400 Bad Request\r\n".to_string(),
            vec!["Connection: close\r\n".to_string()],
            None,
        )
    }

    pub fn length_required() -> Self {
        Self::new(
            "HTTP/1.1 411 Length Required\r\n".to_string(),
            vec!["Connection: close\r\n".to_string()],
            None,
        )
    }

    pub fn created() -> Self {
        Self::new("HTTP/1.1 201 Created\r\n".to_string(), vec![], None)
    }

    pub fn with_content_type(mut self, content_type: &str) -> Self {
        self.headers
            .push(format!("Content-Type: {}\r\n", content_type));
        self
    }

    pub fn with_body(mut self, body: Vec<u8>) -> Self {
        self.headers
            .push(format!("Content-Length: {}\r\n", body.len()));
        self.body = Some(body);
        self
    }

    pub fn send(&self, stream: &mut impl Write) -> std::io::Result<()> {
        stream.write_all(self.status_line.as_bytes())?;
        for header in &self.headers {
            stream.write_all(header.as_bytes())?;
        }
        stream.write_all(b"\r\n")?;
        if let Some(body) = &self.body {
            stream.write_all(body)?;
        }
        stream.flush()?;
        Ok(())
    }
}
