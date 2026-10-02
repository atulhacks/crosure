use crosure_agent::{
    build_chain, openai_request, parse_tool_call, presets, AgentSettings, Block, Entry, KeySource,
    ProviderKind, Stop, ToolResult, Transcript, Turn,
};
use serde_json::json;

fn turn_with_call() -> Turn {
    Turn {
        blocks: vec![
            Block::Thinking("hidden".into()),
            Block::Text("Looking at imports.".into()),
            Block::ToolUse {
                id: "c1".into(),
                name: "list_imports".into(),
                input: json!({"why": "w"}),
            },
        ],
        stop: Stop::ToolUse,
        input_tokens: 0,
        output_tokens: 0,
        raw: json!(null),
    }
}

#[test]
fn openai_request_maps_turns_and_results() {
    let t = Transcript {
        entries: vec![
            Entry::User("task".into()),
            Entry::Assistant {
                provider: "anthropic".into(),
                turn: turn_with_call(),
            },
            Entry::Results {
                results: vec![ToolResult {
                    id: "c1".into(),
                    content: "8 imports".into(),
                    is_error: false,
                }],
                note: Some("wrap up".into()),
            },
        ],
        ..Default::default()
    };
    let body = openai_request("m", &t, false);
    let roles: Vec<&str> = body["messages"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|m| m["role"].as_str())
        .collect();
    assert_eq!(roles, vec!["system", "user", "assistant", "tool", "user"]);
    let a = &body["messages"][2];
    assert_eq!(a["content"], "Looking at imports.");
    assert_eq!(a["tool_calls"][0]["function"]["name"], "list_imports");
    assert_eq!(
        a["tool_calls"][0]["function"]["arguments"],
        "{\"why\":\"w\"}"
    );
    assert_eq!(body["messages"][3]["tool_call_id"], "c1");
    // lenient servers get plain types; strict servers keep the strict schema
    let filter = &body["tools"][1]["function"]["parameters"]["properties"]["filter"]["type"];
    assert_eq!(filter, "string");
    assert!(body["tools"][1]["function"].get("strict").is_none());
    let strict = openai_request("m", &t, true);
    assert_eq!(strict["tools"][1]["function"]["strict"], true);
}

#[test]
fn unparsable_tool_arguments_become_a_clear_error() {
    let bad = parse_tool_call("disassemble", &json!({"__unparsed": "{oops"}));
    assert_eq!(
        bad.err().as_deref(),
        Some("tool arguments were not valid JSON")
    );
    let ok = parse_tool_call("search_strings", &json!({"filter": "", "why": "w"}));
    assert!(ok.is_ok(), "empty optional string means none");
}

#[test]
fn settings_migrate_merge_and_chain() -> Result<(), Box<dyn std::error::Error>> {
    let old = AgentSettings::from_json(br#"{"api_key":"sk-old","model":"claude-opus-5-5"}"#);
    assert_eq!(old.active, "anthropic");
    assert_eq!(old.providers[0].key().1, KeySource::Saved);

    let mut s = old.clone();
    let mut ollama = presets()
        .into_iter()
        .find(|p| p.id == "ollama")
        .ok_or("preset")?;
    ollama.model = "qwen2.5-coder:14b".into();
    assert_eq!(ollama.key().1, KeySource::NotNeeded);
    let mut incoming = s.clone();
    incoming.providers[0].api_key = None; // UI never sees keys: keep the saved one
    incoming.providers.push(ollama);
    incoming.active = "ollama".into();
    s.merge(incoming);
    assert_eq!(s.providers[0].api_key.as_deref(), Some("sk-old"));
    assert!(
        s.views().iter().all(|v| v.config.api_key.is_none()),
        "views never carry keys"
    );

    let chain = build_chain(&s)?;
    let ids: Vec<&str> = chain.iter().map(|p| p.id()).collect();
    assert_eq!(
        ids,
        vec!["ollama", "anthropic"],
        "active first, then fallbacks"
    );
    s.auto_fallback = false;
    assert_eq!(build_chain(&s)?.len(), 1);

    let mut cleared = s.clone();
    let mut inc = s.clone();
    inc.providers[0].api_key = Some(String::new());
    cleared.merge(inc);
    assert_eq!(
        cleared.providers[0].api_key, None,
        "empty string clears a key"
    );
    assert!(presets().iter().any(|p| p.kind == ProviderKind::Anthropic));
    Ok(())
}
