use crate::server::request::Request;
use crate::server::response::Response;
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct Router {
    directory: String,
}

impl Router {
    pub fn new(directory: &str) -> Self {
        Self {
            directory: directory.to_string(),
        }
    }

    pub fn handle_request(&self, request: &Request) -> Response {
        match (request.method(), request.path()) {
            ("GET", "/") => Response::ok(),
            ("GET", "/files/") => self.handle_directory_listing(),
            ("GET", path) if path.starts_with("/echo/") => {
                let echo_str = &path["/echo/".len()..];
                Response::ok()
                    .with_content_type("text/plain")
                    .with_body(echo_str.as_bytes().to_vec())
            }
            ("GET", "/user-agent") => {
                if let Some(user_agent) = request.headers().get("user-agent") {
                    Response::ok()
                        .with_content_type("text/plain")
                        .with_body(user_agent.as_bytes().to_vec())
                } else {
                    Response::ok()
                        .with_content_type("text/plain")
                        .with_body(Vec::new())
                }
            }
            ("GET", path) if path.starts_with("/files/") => {
                self.handle_file_get(&path["/files/".len()..])
            }
            ("POST", path) if path.starts_with("/files/") => {
                self.handle_file_post(&path["/files/".len()..], request.body())
            }
            _ => Response::not_found(),
        }
    }

    fn handle_directory_listing(&self) -> Response {
        match fs::read_dir(&self.directory) {
            Ok(entries) => {
                let mut items = Vec::new();
                for entry in entries.flatten() {
                    if let (Ok(metadata), Some(name_str)) =
                        (entry.metadata(), entry.file_name().to_str())
                    {
                        let type_str = if metadata.is_dir() {
                            "directory"
                        } else if metadata.is_file() {
                            "file"
                        } else {
                            "other"
                        };

                        let modified = metadata
                            .modified()
                            .unwrap_or_else(|_| SystemTime::now())
                            .duration_since(UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs();

                        let size = metadata.len();
                        let size_str = if size < 1024 {
                            format!("{} B", size)
                        } else if size < 1024 * 1024 {
                            format!("{:.1} KB", size as f64 / 1024.0)
                        } else {
                            format!("{:.1} MB", size as f64 / (1024.0 * 1024.0))
                        };

                        items.push(format!(
                            r#"  {{
    "name": "{}",
    "type": "{}",
    "size": "{}",
    "size_bytes": {},
    "modified": {},
    "modified_iso": "{}"
  }}"#,
                            name_str,
                            type_str,
                            size_str,
                            size,
                            modified,
                            chrono::DateTime::<chrono::Utc>::from(
                                SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(modified)
                            )
                            .format("%Y-%m-%dT%H:%M:%SZ")
                        ));
                    }
                }
                let body = format!("[\n{}\n]", items.join(",\n"));
                Response::ok()
                    .with_content_type("application/json")
                    .with_body(body.into_bytes())
            }
            Err(e) => Response::new(
                "HTTP/1.1 500 Internal Server Error\r\n".to_string(),
                vec![],
                Some(format!("Failed to read directory: {}", e).into_bytes()),
            ),
        }
    }

    fn handle_file_get(&self, filename: &str) -> Response {
        if filename.contains("..") || filename.starts_with('/') {
            return Response::bad_request();
        }

        let file_path = Path::new(&self.directory).join(filename);
        match fs::read(&file_path) {
            Ok(contents) => Response::ok()
                .with_content_type("application/octet-stream")
                .with_body(contents),
            Err(_) => Response::not_found(),
        }
    }

    fn handle_file_post(&self, filename: &str, body: Option<&Vec<u8>>) -> Response {
        if filename.contains("..") || filename.starts_with('/') {
            return Response::bad_request();
        }

        if let Some(body_data) = body {
            let file_path = Path::new(&self.directory).join(filename);
            match fs::write(&file_path, body_data) {
                Ok(_) => Response::created(),
                Err(_) => Response::new(
                    "HTTP/1.1 500 Internal Server Error\r\n".to_string(),
                    vec![],
                    Some(b"Failed to write file".to_vec()),
                ),
            }
        } else {
            Response::length_required()
        }
    }
}
