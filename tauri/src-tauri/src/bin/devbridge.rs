//! Dev-only HTTP bridge: serves the app's commands on 127.0.0.1:1421 so the
//! frontend can run in a normal browser (`npm run dev`) for previews and
//! end-to-end tests. Same command code as the desktop app.
//!
//! `cargo run -p crosure --features devbridge --bin crosure-devbridge`

use crosure_lib::{crosure_home, dispatch, AppState};
use tiny_http::{Header, Method, Response, Server};

fn header(k: &str, v: &str) -> Option<Header> {
    Header::from_bytes(k.as_bytes(), v.as_bytes()).ok()
}

fn main() {
    let state = match AppState::open(crosure_home()) {
        Ok(s) => s,
        Err(e) => return eprintln!("devbridge: {e}"),
    };
    let server = match Server::http("127.0.0.1:1421") {
        Ok(s) => s,
        Err(e) => return eprintln!("devbridge: {e}"),
    };
    eprintln!(
        "devbridge listening on http://127.0.0.1:1421 (home {})",
        state.home.display()
    );
    for mut req in server.incoming_requests() {
        let cors = [
            "Access-Control-Allow-Origin",
            "Access-Control-Allow-Headers",
            "Access-Control-Allow-Methods",
        ]
        .into_iter()
        .zip(["*", "content-type", "POST, OPTIONS"])
        .filter_map(|(k, v)| header(k, v));
        let (status, body) = if req.method() == &Method::Options {
            (204, String::new())
        } else {
            let cmd = req.url().trim_start_matches("/invoke/").to_string();
            let mut raw = String::new();
            let _ = req.as_reader().read_to_string(&mut raw);
            let args = serde_json::from_str(&raw).unwrap_or(serde_json::Value::Null);
            match dispatch(&state, &cmd, &args) {
                Ok(v) => (200, v.to_string()),
                Err(e) => (400, serde_json::Value::String(e).to_string()),
            }
        };
        let mut resp = Response::from_string(body).with_status_code(status);
        for h in cors {
            resp.add_header(h);
        }
        if let Some(h) = header("Content-Type", "application/json") {
            resp.add_header(h);
        }
        let _ = req.respond(resp);
    }
}
