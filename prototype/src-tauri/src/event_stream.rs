use crate::runtime::Runtime;
use std::io::{Read, Result as IoResult};
use std::sync::Arc;
use std::time::Duration;
use tiny_http::{Header, Request, Response, StatusCode};

fn header(name: &str, value: &str) -> Header {
    Header::from_bytes(name, value).expect("valid static HTTP header")
}

struct EventStream {
    runtime: Arc<Runtime>,
    cursor: u64,
    buffer: Vec<u8>,
}

impl EventStream {
    fn new(runtime: Arc<Runtime>, cursor: u64) -> Self {
        Self {
            runtime,
            cursor,
            buffer: Vec::new(),
        }
    }
}

impl Read for EventStream {
    fn read(&mut self, target: &mut [u8]) -> IoResult<usize> {
        if target.is_empty() {
            return Ok(0);
        }
        if self.buffer.is_empty() {
            let events = self
                .runtime
                .wait_for_events(self.cursor, Duration::from_secs(15));
            if events.is_empty() {
                self.buffer.extend_from_slice(b": keep-alive\n\n");
            } else {
                for (sequence, payload) in events {
                    self.cursor = sequence;
                    self.buffer.extend_from_slice(
                        format!("id: {sequence}\ndata: {}\n\n", payload).as_bytes(),
                    );
                }
            }
        }
        let length = target.len().min(self.buffer.len());
        target[..length].copy_from_slice(&self.buffer[..length]);
        self.buffer.drain(..length);
        Ok(length)
    }
}

pub(crate) fn respond(request: Request, runtime: Arc<Runtime>, cursor: u64, origin: Option<&str>) {
    let mut headers = vec![
        header("Content-Type", "text/event-stream; charset=utf-8"),
        header("Cache-Control", "no-cache, no-transform"),
        header("X-Accel-Buffering", "no"),
        header("X-Content-Type-Options", "nosniff"),
    ];
    if let Some(origin) = origin {
        headers.push(header("Access-Control-Allow-Origin", origin));
        headers.push(header("Vary", "Origin"));
    }
    let response = Response::new(
        StatusCode(200),
        headers,
        EventStream::new(runtime, cursor),
        None,
        None,
    );
    let _ = request.respond(response);
}
