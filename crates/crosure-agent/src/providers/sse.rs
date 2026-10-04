//! Server-sent events, read on a helper thread so Stop works at once, even
//! while the server is still processing the prompt.
//!
//! The helper sends the request and forwards each `data:` payload over a
//! channel. The agent's thread waits on it in short slices and checks Stop
//! in between. On Stop it drops the channel; the helper ends at its next
//! read and the connection closes, which ends generation on the server.

use std::io::{BufRead, BufReader, Read};
use std::sync::mpsc;
use std::time::Duration;

use serde_json::Value;

use super::http::check;
use crate::{AgentError, Sink};

/// How often the wait checks Stop.
const SLICE: Duration = Duration::from_millis(100);

enum Msg {
    Data(String),
    /// A non-200 reply, already mapped to its error.
    Failed(AgentError),
    End,
}

fn read_body(resp: reqwest::blocking::Response) -> (u16, Option<u64>, Value) {
    let status = resp.status().as_u16();
    let retry_after = resp
        .headers()
        .get("retry-after")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse().ok());
    let mut text = String::new();
    let _ = BufReader::new(resp).take(1 << 20).read_to_string(&mut text);
    let json = serde_json::from_str(&text).unwrap_or(Value::String(text));
    (status, retry_after, json)
}

fn pump(req: reqwest::blocking::RequestBuilder, tx: &mpsc::Sender<Msg>) {
    let resp = match req.send() {
        Ok(r) => r,
        Err(e) => {
            let _ = tx.send(Msg::Failed(AgentError::Network(e.to_string())));
            return;
        }
    };
    if resp.status().as_u16() != 200 {
        let (status, retry_after, json) = read_body(resp);
        let e = check(status, retry_after, json)
            .err()
            .unwrap_or_else(|| AgentError::Protocol("unexpected status".into()));
        let _ = tx.send(Msg::Failed(e));
        return;
    }
    for line in BufReader::new(resp).lines() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                let _ = tx.send(Msg::Failed(AgentError::Network(e.to_string())));
                return;
            }
        };
        let Some(data) = line.strip_prefix("data:") else {
            continue;
        };
        if tx.send(Msg::Data(data.trim().to_string())).is_err() {
            return; // Stopped: dropping the response closes the connection.
        }
    }
    let _ = tx.send(Msg::End);
}

/// Sends `req` and calls `on_data` with each event's `data:` payload until
/// the stream ends. Returns [`AgentError::Stopped`] as soon as the analyst
/// presses Stop.
pub(crate) fn stream(
    req: reqwest::blocking::RequestBuilder,
    sink: &dyn Sink,
    on_data: &mut dyn FnMut(&str) -> Result<(), AgentError>,
) -> Result<(), AgentError> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || pump(req, &tx));
    loop {
        if sink.should_stop() {
            return Err(AgentError::Stopped);
        }
        match rx.recv_timeout(SLICE) {
            Ok(Msg::Data(d)) => on_data(&d)?,
            Ok(Msg::Failed(e)) => return Err(e),
            Ok(Msg::End) | Err(mpsc::RecvTimeoutError::Disconnected) => return Ok(()),
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
    }
}

/// Whether a server refused the request because it cannot stream (some
/// gateways and older local servers): such a provider is then called
/// without streaming.
pub(crate) fn rejects_streaming(e: &AgentError) -> bool {
    matches!(e, AgentError::Api { status: 400 | 404 | 422 | 501, message }
        if message.to_lowercase().contains("stream"))
}

#[cfg(test)]
mod tests {
    use super::rejects_streaming;
    use crate::AgentError;

    #[test]
    fn a_stream_refusal_is_recognized() {
        let api = |status, m: &str| AgentError::Api {
            status,
            message: m.into(),
        };
        assert!(rejects_streaming(&api(400, "Streaming is not supported")));
        assert!(!rejects_streaming(&api(400, "max_tokens too large")));
        assert!(!rejects_streaming(&AgentError::Auth("stream".into())));
    }
}
