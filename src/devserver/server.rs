use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};

use crate::compiler::codegen::build_project;
use crate::config::toml_parser::parse_app_toml;

pub struct DevServer {
    project_dir: PathBuf,
    host: String,
    port: u16,
    listener: TcpListener,
    python_child: Option<Child>,
    python_executable: String,
    entry_file: String,
    hot_reload: bool,
    build_version: u64,
}

impl DevServer {
    pub fn new(project_dir: &Path, host: &str, port: u16) -> Result<Self, String> {
        let mut active_port = port;
        let mut bound_listener = None;

        // Try requested port first, then automatically probe up to 10 fallback ports
        for offset in 0..10 {
            let try_port = port.saturating_add(offset);
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

        let mut python_executable = "python3".to_string();
        let mut entry_file = "app.zvelte".to_string();
        let mut hot_reload = true;
        if let Ok(toml_src) = std::fs::read_to_string(project_dir.join("app.toml")) {
            if let Ok(cfg) = parse_app_toml(&toml_src) {
                if !cfg.python.executable.is_empty() {
                    python_executable = cfg.python.executable;
                }
                if !cfg.project.entry.is_empty() {
                    entry_file = cfg.project.entry;
                }
                hot_reload = cfg.dev.hot_reload;
            }
        }

        Ok(Self {
            project_dir: project_dir.to_path_buf(),
            host: host.to_string(),
            port: active_port,
            listener,
            python_child: None,
            python_executable,
            entry_file,
            hot_reload,
            build_version: 1,
        })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn build_version(&self) -> u64 {
        self.build_version
    }

    fn spawn_worker(&self) -> Option<Child> {
        let worker_path = self.project_dir.join(".zvelte/dev/worker.py");
        if !worker_path.exists() {
            return None;
        }

        let mut candidates = vec![self.python_executable.as_str(), "python3", "python"];
        candidates.dedup();

        for bin in candidates {
            if bin.is_empty() {
                continue;
            }
            if let Ok(child) = Command::new(bin)
                .arg("-u")
                .arg(".zvelte/dev/worker.py")
                .current_dir(&self.project_dir)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .spawn()
            {
                return Some(child);
            }
        }
        None
    }

    fn stop_worker(&mut self) {
        if let Some(mut child) = self.python_child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }

    fn read_mtimes(&self) -> (Option<SystemTime>, Option<SystemTime>) {
        let toml_mtime = std::fs::metadata(self.project_dir.join("app.toml"))
            .and_then(|m| m.modified())
            .ok();
        let entry_mtime = std::fs::metadata(self.project_dir.join(&self.entry_file))
            .and_then(|m| m.modified())
            .ok();
        (toml_mtime, entry_mtime)
    }

    fn check_and_reload(
        &mut self,
        last_mtimes: &mut (Option<SystemTime>, Option<SystemTime>),
    ) {
        let current = self.read_mtimes();
        if current == *last_mtimes {
            return;
        }
        *last_mtimes = current;

        if let Ok(toml_src) = std::fs::read_to_string(self.project_dir.join("app.toml")) {
            if let Ok(cfg) = parse_app_toml(&toml_src) {
                if !cfg.python.executable.is_empty() {
                    self.python_executable = cfg.python.executable;
                }
                if !cfg.project.entry.is_empty() {
                    self.entry_file = cfg.project.entry;
                }
                self.hot_reload = cfg.dev.hot_reload;
            }
        }

        match build_project(&self.project_dir, ".zvelte/dev") {
            Ok(_) => {
                self.build_version += 1;
                self.stop_worker();
                self.python_child = self.spawn_worker();
                println!(
                    "  \x1b[36m↻\x1b[0m Source updated, rebuilt dev bundle (gen #{})",
                    self.build_version
                );
            }
            Err(e) => {
                eprintln!("  \x1b[31m!\x1b[0m Hot reload compile error: {}", e);
            }
        }
    }

    pub fn serve(&mut self, max_requests: Option<usize>) -> Result<(), String> {
        // 1. Initial build to .zvelte/dev
        let _ = build_project(&self.project_dir, ".zvelte/dev")?;

        // 2. Spawn supervised Python worker if worker.py exists
        self.python_child = self.spawn_worker();
        if self.python_child.is_some() {
            println!("  \x1b[32m✔\x1b[0m Python server worker supervisor online");
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
        let mut last_mtimes = self.read_mtimes();
        let mut last_watch_poll = Instant::now();
        self.listener.set_nonblocking(true).map_err(|e| e.to_string())?;

        while running.load(Ordering::SeqCst)
            && (max_requests.is_none() || req_count < max_requests.unwrap())
        {
            if self.hot_reload && last_watch_poll.elapsed() >= Duration::from_millis(200) {
                self.check_and_reload(&mut last_mtimes);
                last_watch_poll = Instant::now();
            }

            match self.listener.accept() {
                Ok((stream, _)) => {
                    // Check reload immediately before handling request so rapid edits before
                    // a client request are compiled deterministically
                    if self.hot_reload {
                        self.check_and_reload(&mut last_mtimes);
                    }
                    req_count += 1;
                    if let Err(e) = self.handle_client(stream) {
                        eprintln!("  ! Request error: {}", e);
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(e) => {
                    eprintln!("  ! Accept error: {}", e);
                }
            }
        }

        self.stop_worker();
        Ok(())
    }

    fn read_http_request(stream: &mut TcpStream) -> Result<String, String> {
        let _ = stream.set_nonblocking(false);
        let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));

        let mut buf = [0u8; 4096];
        let mut req_data = Vec::new();
        let mut header_end = None;

        // Read until complete headers are received
        while header_end.is_none() {
            let n = match stream.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => n,
                Err(e) => {
                    if req_data.is_empty() {
                        return Err(e.to_string());
                    }
                    break;
                }
            };
            req_data.extend_from_slice(&buf[..n]);
            let s = String::from_utf8_lossy(&req_data);
            if let Some(idx) = s.find("\r\n\r\n") {
                header_end = Some((idx, 4));
            } else if let Some(idx) = s.find("\n\n") {
                header_end = Some((idx, 2));
            }
        }

        if let Some((hdr_idx, sep_len)) = header_end {
            let headers_lower = String::from_utf8_lossy(&req_data[..hdr_idx]).to_ascii_lowercase();
            if let Some(cl_idx) = headers_lower.find("content-length:") {
                let after = headers_lower[cl_idx + 15..].trim_start();
                let cl_str: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
                if let Ok(content_len) = cl_str.parse::<usize>() {
                    let body_start = hdr_idx + sep_len;
                    while req_data.len().saturating_sub(body_start) < content_len {
                        let n = stream.read(&mut buf).map_err(|e| e.to_string())?;
                        if n == 0 {
                            break;
                        }
                        req_data.extend_from_slice(&buf[..n]);
                    }
                }
            }
        }

        Ok(String::from_utf8_lossy(&req_data).into_owned())
    }

    fn handle_client(&mut self, mut stream: TcpStream) -> Result<(), String> {
        let req_str = Self::read_http_request(&mut stream)?;
        if req_str.is_empty() {
            return Ok(());
        }

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

        // Static & dev reload file serving
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

        if path == "/__zvelte_version" {
            let body = self.build_version.to_string();
            let header = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/plain; charset=utf-8\r\nCache-Control: no-store\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            stream.write_all(header.as_bytes()).map_err(|e| e.to_string())?;
            stream.write_all(body.as_bytes()).map_err(|e| e.to_string())?;
        } else if path == "/" || path == "/index.html" {
            let html_path = dev_dir.join("index.html");
            if let Ok(mut content) = std::fs::read_to_string(&html_path) {
                if self.hot_reload {
                    let reload_script = r#"  <script>(function(){var v=null;setInterval(function(){fetch("/__zvelte_version").then(function(r){return r.text()}).then(function(n){if(v!==null&&v!==n){location.reload()}v=n}).catch(function(){})},500);})();</script>
</body>"#;
                    if content.contains("</body>") {
                        content = content.replacen("</body>", reload_script, 1);
                    }
                }
                let bytes = content.as_bytes();
                let header = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nCache-Control: no-store\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    bytes.len()
                );
                stream.write_all(header.as_bytes()).map_err(|e| e.to_string())?;
                stream.write_all(bytes).map_err(|e| e.to_string())?;
            } else {
                stream.write_all(b"HTTP/1.1 500 Internal Error\r\n\r\n").map_err(|e| e.to_string())?;
            }
        } else if path == "/app.js" {
            let js_path = dev_dir.join("app.js");
            if let Ok(content) = std::fs::read(&js_path) {
                let header = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/javascript; charset=utf-8\r\nCache-Control: no-store\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
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
        self.stop_worker();
    }
}

fn ctrlc_handler<F>(f: F) -> Result<(), String>
where
    F: FnMut() + Send + 'static,
{
    let _ = std::thread::spawn(move || {
        let _ = f;
    });
    Ok(())
}
