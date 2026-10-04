//! A one-request HTTP server for exercising the providers' real HTTP paths.
#![allow(clippy::expect_used)] // the mock server thread fails the test loudly on I/O errors

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::mpsc;

use serde_json::Value;

/// Serves one request with `reply`, returns (request line + headers, body).
pub fn serve_once(reply: Value) -> (String, mpsc::Receiver<(String, Value)>) {
    serve_status(reply, "200 OK", "")
}

/// Like [`serve_once`] with a chosen status line and extra header lines.
pub fn serve_status(
    reply: Value,
    status: &'static str,
    headers: &'static str,
) -> (String, mpsc::Receiver<(String, Value)>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = format!("http://{}", listener.local_addr().expect("addr"));
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let (stream, _) = listener.accept().expect("accept");
        let mut reader = BufReader::new(stream.try_clone().expect("clone"));
        let mut head = String::new();
        let mut len = 0usize;
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).expect("line");
            if line == "\r\n" || line.is_empty() {
                break;
            }
            if let Some(v) = line.to_lowercase().strip_prefix("content-length:") {
                len = v.trim().parse().unwrap_or(0);
            }
            head.push_str(&line);
        }
        let mut body = vec![0; len];
        reader.read_exact(&mut body).expect("body");
        let payload = reply.to_string();
        let mut stream = stream;
        write!(stream, "HTTP/1.1 {status}\r\ncontent-type: application/json\r\n{headers}content-length: {}\r\nconnection: close\r\n\r\n{}", payload.len(), payload).expect("write");
        tx.send((head, serde_json::from_slice(&body).unwrap_or(Value::Null)))
            .expect("send");
    });
    (addr, rx)
}

/// One canned HTTP reply for [`serve_seq`].
pub struct Reply {
    pub status: &'static str,
    pub content_type: &'static str,
    pub body: String,
    /// Send only this many body bytes, then hold the connection open.
    pub hang_after: Option<usize>,
}

impl Reply {
    /// A `200 OK` server-sent event stream with these `data:` payloads.
    pub fn sse(events: &[&str]) -> Self {
        Self {
            status: "200 OK",
            content_type: "text/event-stream",
            body: events.iter().map(|e| format!("data: {e}\n\n")).collect(),
            hang_after: None,
        }
    }

    /// A JSON reply.
    pub fn json(status: &'static str, v: &Value) -> Self {
        Self {
            status,
            content_type: "application/json",
            body: v.to_string(),
            hang_after: None,
        }
    }
}

/// Serves `replies` to successive requests; returns each request's body.
pub fn serve_seq(replies: Vec<Reply>) -> (String, mpsc::Receiver<Value>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = format!("http://{}", listener.local_addr().expect("addr"));
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        for reply in replies {
            let Ok((stream, _)) = listener.accept() else {
                return;
            };
            let mut reader = BufReader::new(stream.try_clone().expect("clone"));
            let mut len = 0usize;
            loop {
                let mut line = String::new();
                reader.read_line(&mut line).expect("line");
                if line == "\r\n" || line.is_empty() {
                    break;
                }
                if let Some(v) = line.to_lowercase().strip_prefix("content-length:") {
                    len = v.trim().parse().unwrap_or(0);
                }
            }
            let mut body = vec![0; len];
            reader.read_exact(&mut body).expect("body");
            let _ = tx.send(serde_json::from_slice(&body).unwrap_or(Value::Null));
            let mut stream = stream;
            let head = match reply.hang_after {
                // No length: the body ends when the connection closes.
                Some(_) => format!("HTTP/1.1 {}\r\ncontent-type: {}\r\nconnection: close\r\n\r\n", reply.status, reply.content_type),
                None => format!("HTTP/1.1 {}\r\ncontent-type: {}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n", reply.status, reply.content_type, reply.body.len()),
            };
            let _ = stream.write_all(head.as_bytes());
            match reply.hang_after {
                Some(n) => {
                    let _ = stream.write_all(&reply.body.as_bytes()[..n.min(reply.body.len())]);
                    let _ = stream.flush();
                    std::thread::sleep(std::time::Duration::from_secs(5));
                }
                None => {
                    let _ = stream.write_all(reply.body.as_bytes());
                }
            }
        }
    });
    (addr, rx)
}
