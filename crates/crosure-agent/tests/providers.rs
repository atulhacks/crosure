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
        cache_read_tokens: 0,
        cache_write_tokens: 0,
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
    let body = openai_request("p", "m", &t, false);
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
    let strict = openai_request("p", "m", &t, true);
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

#[test]
fn reasoning_and_thought_signatures_go_back_to_their_own_provider() {
    let mut turn = turn_with_call();
    turn.raw = json!({
        "reasoning_content": "Imports first.",
        "tool_calls": [{ "id": "c1", "extra_content": { "google": { "thought_signature": "sig" } } }]
    });
    let t = Transcript {
        entries: vec![
            Entry::User("task".into()),
            Entry::Assistant {
                provider: "moonshot".into(),
                turn,
            },
            Entry::Results {
                results: vec![ToolResult {
                    id: "c1".into(),
                    content: "8 imports".into(),
                    is_error: false,
                }],
                note: None,
            },
        ],
        ..Default::default()
    };
    let own = openai_request("moonshot", "kimi-k2-thinking", &t, false);
    let a = &own["messages"][2];
    assert_eq!(a["reasoning_content"], "Imports first.");
    assert_eq!(
        a["tool_calls"][0]["extra_content"]["google"]["thought_signature"],
        "sig"
    );
    // after a fallback, another provider never sees it
    let other = openai_request("openai", "gpt", &t, true);
    assert!(other["messages"][2].get("reasoning_content").is_none());
    assert!(other["messages"][2]["tool_calls"][0]
        .get("extra_content")
        .is_none());
}

#[test]
fn presets_cover_hosted_and_local_providers() {
    let p = presets();
    for id in [
        "anthropic",
        "openai",
        "gemini",
        "deepseek",
        "zai",
        "moonshot",
        "xai",
        "qwen",
        "ollama",
        "lmstudio",
        "llamacpp",
        "vllm",
    ] {
        assert!(p.iter().any(|x| x.id == id), "missing preset {id}");
    }
    let mut ids: Vec<&str> = p.iter().map(|x| x.id.as_str()).collect();
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), p.len(), "preset ids are unique");
    // a custom Anthropic-compatible endpoint is not ready until it has a URL
    let mut c = p
        .iter()
        .find(|x| x.id == "custom-anthropic")
        .cloned()
        .unwrap_or_else(|| p[0].clone());
    c.api_key = Some("k".into());
    c.model = "glm-4.6".into();
    assert!(!c.ready());
    c.base_url = "https://api.z.ai/api/anthropic".into();
    assert!(c.ready());
}

#[test]
fn openrouter_reasoning_details_and_openai_output_limit() {
    let mut turn = turn_with_call();
    turn.raw = json!({ "reasoning_details": [{ "type": "reasoning.encrypted", "data": "x" }] });
    let t = Transcript {
        entries: vec![
            Entry::User("task".into()),
            Entry::Assistant {
                provider: "openrouter".into(),
                turn,
            },
        ],
        ..Default::default()
    };
    let body = openai_request("openrouter", "m", &t, false);
    assert_eq!(body["messages"][2]["reasoning_details"][0]["data"], "x");

    let p = presets();
    let openai = p
        .iter()
        .find(|x| x.id == "openai")
        .cloned()
        .unwrap_or_else(|| p[0].clone());
    assert!(
        openai.extras().max_completion_tokens,
        "api.openai.com needs max_completion_tokens"
    );
    let deepseek = p
        .iter()
        .find(|x| x.id == "deepseek")
        .cloned()
        .unwrap_or_else(|| p[0].clone());
    assert!(!deepseek.extras().max_completion_tokens);
    let mut custom = p
        .iter()
        .find(|x| x.id == "custom")
        .cloned()
        .unwrap_or_else(|| p[0].clone());
    custom.id = "my-gateway".into();
    assert_eq!(
        custom.key_env_name(),
        "CUSTOM_API_KEY".replace("CUSTOM", "MY_GATEWAY")
    );
}

#[test]
fn old_settings_without_new_fields_still_load() {
    let s = AgentSettings::from_json(
        br#"{"providers":[{"id":"zai","kind":"openai_compatible","label":"Z","base_url":"https://api.z.ai/api/paas/v4","model":"glm-4.6","key_env":"ZAI_API_KEY","strict_tools":false,"enabled":true}],"active":"zai"}"#,
    );
    let p = &s.providers[0];
    assert_eq!(p.id, "zai");
    assert!(p.headers.is_empty() && p.reasoning_effort.is_none() && !p.max_completion_tokens);
}

#[test]
fn tool_schemas_match_the_snapshot() -> Result<(), Box<dyn std::error::Error>> {
    // Tool and field names are in recorded datasets and prompts: any change
    // to this snapshot must be deliberate.
    let snapshot: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/tool_definitions.json"))?;
    assert_eq!(crosure_agent::tool_definitions(), snapshot);
    Ok(())
}

#[test]
fn every_tool_call_maps_to_the_same_op_as_its_command() -> Result<(), Box<dyn std::error::Error>> {
    for spec in crosure_session::OPS {
        let mut input = serde_json::Map::new();
        input.insert("why".into(), json!("test"));
        let mut line = vec![spec.command.to_string()];
        for a in spec.args {
            let (v, w) = match a.kind {
                crosure_session::ArgKind::Count { .. } => (json!(64), "64".to_string()),
                crosure_session::ArgKind::Choice(o) => (json!(o[0]), o[0].to_string()),
                crosure_session::ArgKind::Text => (json!("some text"), "some text".into()),
                _ => (json!("check_password"), "check_password".into()),
            };
            if let Some(f) = a.tool_field {
                input.insert(f.into(), v);
                match a.placement {
                    crosure_session::Placement::Positional => line.push(w),
                    crosure_session::Placement::Flag(flag) => {
                        line.insert(1, w);
                        line.insert(1, flag.to_string());
                    }
                }
            }
        }
        let call = parse_tool_call(spec.tool, &serde_json::Value::Object(input))?;
        assert_eq!(
            call.op,
            crosure_session::parse_command(&line.join(" "))?,
            "{}",
            spec.tool
        );
    }
    Ok(())
}
