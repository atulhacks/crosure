use std::sync::Mutex;

use serde_json::{json, Value};

use crate::{AgentError, Llm};

/// A canned model for tests and offline demos: replays fixed responses in order.
/// Every response it returns is clearly labelled as scripted in its text.
pub struct ScriptedLlm {
    responses: Mutex<Vec<Value>>,
    /// Request bodies received, for assertions.
    pub requests: Mutex<Vec<Value>>,
}

impl ScriptedLlm {
    /// Replays `responses` (Messages API response bodies) one per call.
    pub fn new(responses: Vec<Value>) -> Self {
        let mut r = responses;
        r.reverse();
        Self {
            responses: Mutex::new(r),
            requests: Mutex::new(Vec::new()),
        }
    }

    /// A response that calls one tool.
    pub fn tool(id: &str, name: &str, input: Value, thought: &str) -> Value {
        json!({
            "stop_reason": "tool_use",
            "content": [
                { "type": "thinking", "thinking": thought, "signature": "scripted" },
                { "type": "tool_use", "id": id, "name": name, "input": input }
            ],
            "usage": { "input_tokens": 1000, "output_tokens": 80 }
        })
    }

    /// A final text response.
    pub fn done(text: &str) -> Value {
        json!({
            "stop_reason": "end_turn",
            "content": [{ "type": "text", "text": text }],
            "usage": { "input_tokens": 1500, "output_tokens": 300 }
        })
    }

    /// A walkthrough of the bundled crackme fixture (used by the offline demo).
    pub fn crackme_demo() -> Self {
        let t = Self::tool;
        Self::new(vec![
            t("t1", "binary_info", json!({"why": "Establish format and architecture before anything else."}), "Start with the basics."),
            t("t2", "list_imports", json!({"why": "Imports hint at behaviour: string compares, I/O, crypto, network."}), "Imports next."),
            t("t3", "search_strings", json!({"filter": null, "why": "Strings often reveal prompts, secrets and C2 indicators."}), "Look at the strings."),
            t("t4", "xrefs_to", json!({"target": "strcmp@plt", "why": "A strcmp in a crackme usually guards the password check."}), "strcmp is the obvious lead."),
            t("t5", "disassemble", json!({"target": "check_password", "why": "This is the strcmp caller; see what it compares the input against."}), "Read the checker."),
            t("t6", "record_hypothesis", json!({"text": "The expected password is stored encoded and decoded by the function called before strcmp.", "why": "Pin the idea before verifying it."}), "Form a hypothesis."),
            t("t7", "disassemble", json!({"target": "decode", "why": "Confirm how the stored secret is transformed."}), "Check the decoder."),
            t("t8", "rename_function", json!({"target": "decode", "new_name": "xor_decode", "why": "It XORs each byte with a key; name it for what it does."}), "Rename."),
            t("t9", "record_finding", json!({"text": "check_password XOR-decodes a 7-byte secret with key 0x43 and strcmp's it with argv[1].", "why": "Evidence from check_password and xor_decode confirms the hypothesis."}), "Confirmed."),
            t("t10", "record_verdict", json!({"verdict": "benign", "family": null, "summary": "A password crackme; the URL is a placeholder and nothing is sent anywhere.", "why": "Close the investigation."}), "Wrap up."),
            Self::done("*(Scripted demo model, not Claude)*\n\n## Summary\nA small crackme: `main` checks argc, calls `check_password(argv[1])`, prints a success line and a beacon URL.\n\n## Key functions\n| Address | Name | Role |\n|---|---|---|\n| 0x11d9 | check_password | decodes the secret, `strcmp`s it with the input |\n| 0x1189 | xor_decode | XORs a buffer with a 1-byte key (0x43) |\n\n## Indicators\n- `http://c2.example.invalid/beacon` (placeholder, printed only)\n\n## Verdict\nBenign, high confidence.\n\n## Open questions\n- None for static analysis."),
        ])
    }
}

impl Llm for ScriptedLlm {
    fn create(&self, body: &Value) -> Result<Value, AgentError> {
        if let Ok(mut r) = self.requests.lock() {
            r.push(body.clone());
        }
        self.responses
            .lock()
            .map_err(|_| AgentError::Protocol("script lock poisoned".into()))?
            .pop()
            .ok_or_else(|| AgentError::Protocol("script exhausted".into()))
    }

    fn model(&self) -> &str {
        "scripted-demo"
    }
}
