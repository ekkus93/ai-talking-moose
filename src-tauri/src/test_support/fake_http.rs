//! Thread-backed loopback HTTP fake for provider-routing tests.
//!
//! It runs on its own OS thread so it works under both `#[tokio::test]` and the
//! synchronous Tauri mock-IPC tests (whose command future blocks the test thread).
//! Every connection is recorded, so "this provider was never contacted" is
//! asserted as an empty hit list rather than inferred from a timeout.
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Clone, Debug)]
pub(crate) struct Hit {
    pub request_line: String,
    pub body: String,
}

pub(crate) struct FakeHttp {
    pub address: String,
    hits: Arc<Mutex<Vec<Hit>>>,
    connections: Arc<Mutex<usize>>,
    stop: Arc<AtomicBool>,
}

impl FakeHttp {
    /// Answers every request with `status_line` (e.g. `HTTP/1.1 200 OK`) and `body`.
    pub fn spawn(status_line: &'static str, body: &'static str) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let address = listener.local_addr().unwrap().to_string();
        let hits = Arc::new(Mutex::new(Vec::new()));
        let connections = Arc::new(Mutex::new(0usize));
        let stop = Arc::new(AtomicBool::new(false));
        let (t_hits, t_conn, t_stop) = (hits.clone(), connections.clone(), stop.clone());
        std::thread::spawn(move || {
            while !t_stop.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        *t_conn.lock().unwrap() += 1;
                        stream.set_nonblocking(false).unwrap();
                        stream.set_read_timeout(Some(Duration::from_secs(2))).ok();
                        let mut raw = Vec::new();
                        let mut buf = [0u8; 8192];
                        while let Ok(n) = stream.read(&mut buf) {
                            raw.extend_from_slice(&buf[..n]);
                            let text = String::from_utf8_lossy(&raw).to_string();
                            if let Some(split) = text.find("\r\n\r\n") {
                                let length = text[..split]
                                    .lines()
                                    .find_map(|l| {
                                        l.to_ascii_lowercase()
                                            .strip_prefix("content-length:")
                                            .and_then(|v| v.trim().parse::<usize>().ok())
                                    })
                                    .unwrap_or(0);
                                if raw.len() >= split + 4 + length {
                                    break;
                                }
                            }
                            if n == 0 {
                                break;
                            }
                        }
                        let text = String::from_utf8_lossy(&raw).to_string();
                        if let Some((head, req_body)) = text.split_once("\r\n\r\n") {
                            t_hits.lock().unwrap().push(Hit {
                                request_line: head.lines().next().unwrap_or("").to_string(),
                                body: req_body.to_string(),
                            });
                        }
                        let response = format!(
                            "{status_line}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                            body.len()
                        );
                        let _ = stream.write_all(response.as_bytes());
                        let _ = stream.flush();
                    }
                    Err(_) => std::thread::sleep(Duration::from_millis(5)),
                }
            }
        });
        Self {
            address,
            hits,
            connections,
            stop,
        }
    }

    /// `http://127.0.0.1:<port>` with no path, as the loopback settings expect.
    pub fn origin(&self) -> String {
        format!("http://{}", self.address)
    }

    pub fn hits(&self) -> Vec<Hit> {
        self.hits.lock().unwrap().clone()
    }

    /// Raw accepted TCP connections, including ones that sent no parseable request.
    pub fn connection_count(&self) -> usize {
        *self.connections.lock().unwrap()
    }
}

impl Drop for FakeHttp {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
    }
}

pub(crate) const OLLAMA_OK: &str = r#"{"done":true,"model":"selected-model","message":{"role":"assistant","content":"Ollama says hi."}}"#;
pub(crate) const LLAMA_OK: &str = r#"{"model":"selected-model","choices":[{"finish_reason":"stop","message":{"role":"assistant","content":"llama-server says hi."}}]}"#;
pub(crate) const OPENAI_OK: &str = r#"{"status":"completed","output":[{"type":"message","role":"assistant","content":[{"type":"output_text","text":"OpenAI says hi."}]}]}"#;
pub(crate) const UNAVAILABLE: &str = "HTTP/1.1 503 Service Unavailable";
pub(crate) const OK: &str = "HTTP/1.1 200 OK";
