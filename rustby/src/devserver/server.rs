use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use crate::compiler::codegen::build_project;

pub struct DevServer {
    project_dir: PathBuf,
    host: String,
    port: u16,
    listener: TcpListener,
    python_child: Option<Child>,
}

impl DevServer {
    pub fn new(project_dir: &Path, host: &str, port: u16) -> Result<Self, String> {
        let mut active_port = port;
        let mut bound_listener = None;

        // Try requested port first, then automatically probe up to 10 fallback ports
        for offset in 0..10 {
            let try_port = port + offset;
            let addr = format!("{}:{}", host, try_port);
            if let Ok(listener) = TcpListener::bind(&addr) {
                active_port = try_port;
                bound_listener = Some(listener);
                if offset > 0 {
                    eprintln!(
                        "  \x1b[33m!\x1b[0m Port {} is in use, automatically switched to \x1b[1m{}\x1b[0m",
                        port, active_port
                    );
                }
                break;
            }
        }

        let listener = bound_listener.ok_or_else(|| {
            format!(
                "Port {} (and nearby fallback ports) are already in use on {}",
                port, host
            )
        })?;

        Ok(Self {
            project_dir: project_dir.to_path_buf(),
            host: host.to_string(),
            port: active_port,
            listener,
            python_child: None,
        })
    }

    pub fn serve(&mut self, max_requests: Option<usize>) -> Result<(), String> {
        // 1. Initial build to .zvelte/dev (or .rustby/dev)
        let _ = build_project(&self.project_dir, ".zvelte/dev")?;

        // 2. Spawn supervised Python worker if worker.py exists
        let dev_dir = self.project_dir.join(".zvelte/dev");
        let worker_path = dev_dir.join("worker.py");
        if worker_path.exists() {
            // Try python3 first, fallback to python on Windows
            let child = Command::new("python3")
                .arg("-u")
                .arg(".zvelte/dev/worker.py")
                .current_dir(&self.project_dir)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .spawn()
                .or_else(|_| {
                    Command::new("python")
                        .arg("-u")
                        .arg(".zvelte/dev/worker.py")
                        .current_dir(&self.project_dir)
                        .stdin(Stdio::piped())
                        .stdout(Stdio::piped())
                        .spawn()
                })
                .ok();

            if child.is_some() {
                println!("  \x1b[32m✔\x1b[0m Python server worker supervisor online");
            }
            self.python_child = child;
        }

        println!(
            "\n  \x1b[32m✔\x1b[0m Zvelte dev server running at \x1b[1mhttp://{}:{}/\x1b[0m\n",
            self.host, self.port
        );

        let running = Arc::new(AtomicBool::new(true));
        let r = running.clone();
        let _ = ctrlc_handler(move || {
            r.store(false, Ordering::SeqCst);
        });

        let mut req_count = 0;
        self.listener.set_nonblocking(true).map_err(|e| e.to_string())?;

        while running.load(Ordering::SeqCst) && (max_requests.is_none() || req_count < max_requests.unwrap()) {
            match self.listener.accept() {
                Ok((stream, _)) => {
                    req_count += 1;
                    if let Err(e) = self.handle_client(stream) {
                        eprintln!("  ! Request error: {}", e);
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                Err(e) => {
                    eprintln!("  ! Accept error: {}", e);
                }
            }
        }

        // Cleanup
        if let Some(mut child) = self.python_child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }

        Ok(())
    }

    fn handle_client(&mut self, mut stream: TcpStream) -> Result<(), String> {
        let mut buf = [0u8; 4096];
        let bytes_read = stream.read(&mut buf).map_err(|e| e.to_string())?;
        if bytes_read == 0 {
            return Ok(());
        }

        let mut req_data = Vec::from(&buf[..bytes_read]);

        // Check Content-Length for complete body ingestion
        let req_str = String::from_utf8_lossy(&req_data);
        if let Some(cl_idx) = req_str.find("Content-Length: ") {
            let after = &req_str[cl_idx + 16..];
            let cl_str: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
            if let Ok(content_len) = cl_str.parse::<usize>() {
                let body_start = if let Some(idx) = req_str.find("\r\n\r\n") {
                    idx + 4
                } else if let Some(idx) = req_str.find("\n\n") {
                    idx + 2
                } else {
                    req_str.len()
                };

                let current_body_len = req_data.len().saturating_sub(body_start);
                if current_body_len < content_len {
                    let mut remaining = content_len - current_body_len;
                    while remaining > 0 {
                        let n = stream.read(&mut buf).map_err(|e| e.to_string())?;
                        if n == 0 {
                            break;
                        }
                        req_data.extend_from_slice(&buf[..n]);
                        remaining = remaining.saturating_sub(n);
                    }
                }
            }
        }

        let req_str = String::from_utf8_lossy(&req_data);

        // CORS preflight
        if req_str.starts_with("OPTIONS ") {
            let cors = "HTTP/1.1 204 No Content\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Methods: POST, GET, OPTIONS\r\nAccess-Control-Allow-Headers: Content-Type\r\nConnection: close\r\n\r\n";
            stream.write_all(cors.as_bytes()).map_err(|e| e.to_string())?;
            return Ok(());
        }

        // Python Server RPC Bridge
        if req_str.starts_with("POST /api/rpc") {
            if self.python_child.is_none() {
                let err_body = r#"{"error": "Python server worker is not running"}"#;
                let header = format!(
                    "HTTP/1.1 503 Service Unavailable\r\nContent-Type: application/json\r\nAccess-Control-Allow-Origin: *\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    err_body.len()
                );
                stream.write_all(header.as_bytes()).map_err(|e| e.to_string())?;
                stream.write_all(err_body.as_bytes()).map_err(|e| e.to_string())?;
                return Ok(());
            }

            let body_start = if let Some(idx) = req_str.find("\r\n\r\n") {
                idx + 4
            } else if let Some(idx) = req_str.find("\n\n") {
                idx + 2
            } else {
                req_str.len()
            };

            let req_body = req_str[body_start..].trim();

            let child = self.python_child.as_mut().unwrap();
            if let Some(stdin) = child.stdin.as_mut() {
                stdin.write_all(req_body.as_bytes()).map_err(|e| e.to_string())?;
                stdin.write_all(b"\n").map_err(|e| e.to_string())?;
                stdin.flush().map_err(|e| e.to_string())?;
            }

            let mut resp_line = String::new();
            if let Some(stdout) = child.stdout.as_mut() {
                let mut reader = BufReader::new(stdout);
                let _ = reader.read_line(&mut resp_line);
            }

            let resp_trimmed = resp_line.trim();
            let resp_body = if !resp_trimmed.is_empty() {
                resp_trimmed
            } else {
                r#"{"error": "Empty response from Python worker"}"#
            };

            let header = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json; charset=utf-8\r\nAccess-Control-Allow-Origin: *\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                resp_body.len()
            );
            stream.write_all(header.as_bytes()).map_err(|e| e.to_string())?;
            stream.write_all(resp_body.as_bytes()).map_err(|e| e.to_string())?;
            return Ok(());
        }

        // Static file serving
        let path = if req_str.starts_with("GET ") {
            let start = 4;
            let end = req_str[start..]
                .find(|c: char| c.is_whitespace())
                .map(|i| start + i)
                .unwrap_or(req_str.len());
            &req_str[start..end]
        } else {
            "/"
        };

        let dev_dir = self.project_dir.join(".zvelte/dev");

        if path == "/" || path == "/index.html" {
            let html_path = dev_dir.join("index.html");
            if let Ok(content) = std::fs::read(&html_path) {
                let header = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    content.len()
                );
                stream.write_all(header.as_bytes()).map_err(|e| e.to_string())?;
                stream.write_all(&content).map_err(|e| e.to_string())?;
            } else {
                stream.write_all(b"HTTP/1.1 500 Internal Error\r\n\r\n").map_err(|e| e.to_string())?;
            }
        } else if path == "/app.js" {
            let js_path = dev_dir.join("app.js");
            if let Ok(content) = std::fs::read(&js_path) {
                let header = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/javascript; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    content.len()
                );
                stream.write_all(header.as_bytes()).map_err(|e| e.to_string())?;
                stream.write_all(&content).map_err(|e| e.to_string())?;
            } else {
                stream.write_all(b"HTTP/1.1 500 Internal Error\r\n\r\n").map_err(|e| e.to_string())?;
            }
        } else {
            let not_found = "HTTP/1.1 404 Not Found\r\nContent-Type: text/plain\r\nContent-Length: 9\r\nConnection: close\r\n\r\nNot Found";
            stream.write_all(not_found.as_bytes()).map_err(|e| e.to_string())?;
        }

        Ok(())
    }
}

impl Drop for DevServer {
    fn drop(&mut self) {
        if let Some(mut child) = self.python_child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

fn ctrlc_handler<F>(f: F) -> Result<(), String>
where
    F: FnMut() + Send + 'static,
{
    // Minimal standard signal handler or no-op if unsupported
    let _ = std::thread::spawn(move || {
        // Keeps hook available if needed
        let _ = f;
    });
    Ok(())
}
