use crate::work::PowWorkTemplate;
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;
use tracing::{error, info, warn};

#[derive(Clone)]
pub struct WorkManager {
    inner: Arc<RwLock<WorkState>>,
}

#[derive(Clone)]
struct WorkState {
    generation: u64,
    template: PowWorkTemplate,
}

impl WorkManager {
    pub fn new(template: PowWorkTemplate) -> Self {
        Self {
            inner: Arc::new(RwLock::new(WorkState {
                generation: 0,
                template,
            })),
        }
    }

    pub fn current(&self) -> (u64, PowWorkTemplate) {
        let state = self.inner.read().expect("work state poisoned");
        (state.generation, state.template.clone())
    }

    pub fn replace(&self, template: PowWorkTemplate) {
        let mut state = self.inner.write().expect("work state poisoned");
        state.generation = state.generation.saturating_add(1);
        state.template = template;
    }

    pub fn is_stale(&self, generation: u64, template_id: &[u8; 32]) -> bool {
        let state = self.inner.read().expect("work state poisoned");
        generation != state.generation || &state.template.template_id() != template_id
    }
}

pub fn spawn(
    bind: SocketAddr,
    work: WorkManager,
    running: Arc<AtomicBool>,
) -> Result<thread::JoinHandle<()>, String> {
    let listener =
        TcpListener::bind(bind).map_err(|e| format!("failed to bind mining RPC on {bind}: {e}"))?;
    listener
        .set_nonblocking(true)
        .map_err(|e| format!("failed to configure mining RPC listener: {e}"))?;

    info!(%bind, "mining RPC listening");

    Ok(thread::spawn(move || {
        while running.load(Ordering::SeqCst) {
            match listener.accept() {
                Ok((stream, peer)) => {
                    if let Err(e) = handle_connection(stream, &work) {
                        warn!(%peer, error = %e, "mining RPC request failed");
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(25));
                }
                Err(e) => {
                    error!(error = %e, "mining RPC accept failed");
                    thread::sleep(Duration::from_millis(100));
                }
            }
        }
    }))
}

fn handle_connection(mut stream: TcpStream, work: &WorkManager) -> Result<(), String> {
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .map_err(|e| format!("failed to set RPC read timeout: {e}"))?;

    let mut buf = [0_u8; 16 * 1024];
    let size = stream
        .read(&mut buf)
        .map_err(|e| format!("failed to read RPC request: {e}"))?;

    if size == 0 {
        return Err("empty RPC request".into());
    }

    let request_text = String::from_utf8_lossy(&buf[..size]);
    let body = request_text
        .split_once("\r\n\r\n")
        .map(|(_, body)| body)
        .ok_or_else(|| "malformed HTTP request".to_string())?;

    let request: Value =
        serde_json::from_str(body).map_err(|e| format!("invalid JSON-RPC request: {e}"))?;

    let id = request.get("id").cloned().unwrap_or(Value::Null);
    let method = request
        .get("method")
        .and_then(Value::as_str)
        .ok_or_else(|| "JSON-RPC method missing".to_string())?;

    let response = match method {
        "pow_getWork" => {
            let (generation, template) = work.current();
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "development": true,
                    "generation": generation,
                    "template_id": hex::encode(template.template_id()),
                    "version": template.version,
                    "height": template.height,
                    "parent_hash": hex::encode(template.parent_hash),
                    "execution_commitment": hex::encode(template.execution_commitment),
                    "timestamp": template.timestamp,
                    "difficulty": template.difficulty,
                    "target": hex::encode(template.target),
                    "nonce_start": 0_u64,
                    "nonce_end": u64::MAX
                }
            })
        }
        _ => json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {
                "code": -32601,
                "message": "method not found"
            }
        }),
    };

    let payload = serde_json::to_vec(&response)
        .map_err(|e| format!("failed to encode JSON-RPC response: {e}"))?;

    let headers = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        payload.len()
    );

    stream
        .write_all(headers.as_bytes())
        .and_then(|_| stream.write_all(&payload))
        .map_err(|e| format!("failed to write RPC response: {e}"))
}

#[cfg(test)]
mod tests {
    use super::WorkManager;
    use crate::work::PowWorkTemplate;

    fn template(marker: u8) -> PowWorkTemplate {
        PowWorkTemplate {
            version: 1,
            height: marker as u64,
            parent_hash: [marker; 32],
            execution_commitment: [marker.wrapping_add(1); 32],
            timestamp: 1_800_000_000 + marker as u64,
            difficulty: 1,
            target: [0xff; 32],
        }
    }

    #[test]
    fn replacing_work_marks_previous_generation_stale() {
        let manager = WorkManager::new(template(1));
        let (generation, current) = manager.current();
        let old_id = current.template_id();

        assert!(!manager.is_stale(generation, &old_id));

        manager.replace(template(2));

        assert!(manager.is_stale(generation, &old_id));
    }
}
