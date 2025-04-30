use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::net::TcpStream;

pub fn parse_headers(
    reader: &mut BufReader<&TcpStream>,
) -> std::io::Result<HashMap<String, String>> {
    let mut headers = HashMap::new();
    let mut header_line = String::new();

    loop {
        let bytes_read = reader.read_line(&mut header_line)?;
        if bytes_read == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "Connection closed while reading headers",
            ));
        }

        let trimmed_line = header_line.trim();
        if trimmed_line.is_empty() {
            break;
        }

        if let Some((name, value)) = trimmed_line.split_once(": ") {
            headers.insert(name.to_lowercase(), value.trim().to_string());
        } else {
            println!("Warning: Malformed header line ignored: {}", trimmed_line);
        }
        header_line.clear();
    }
    Ok(headers)
}
