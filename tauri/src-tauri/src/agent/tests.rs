use super::AgentRuntime;

#[test]
fn setup_is_shown_until_a_provider_is_chosen() -> Result<(), String> {
    let home = std::env::temp_dir().join(format!("crosure-first-run-{}", std::process::id()));
    std::fs::create_dir_all(&home).map_err(|e| e.to_string())?;
    let _ = std::fs::remove_file(home.join("agent.json"));

    let rt = AgentRuntime::new(&home);
    let before = rt.status();
    assert!(before.first_run, "nothing saved yet");
    assert_eq!(before.key_env.as_deref(), Some("ANTHROPIC_API_KEY"));

    let after = rt.set_active(&before.provider)?;
    assert!(!after.first_run, "choosing a provider saves the settings");
    assert!(
        !AgentRuntime::new(&home).status().first_run,
        "and survives a restart"
    );
    std::fs::remove_dir_all(&home).map_err(|e| e.to_string())
}

#[test]
fn a_preset_can_be_chosen_before_it_is_configured() -> Result<(), String> {
    let home = std::env::temp_dir().join(format!("crosure-adopt-{}", std::process::id()));
    std::fs::create_dir_all(&home).map_err(|e| e.to_string())?;
    let _ = std::fs::remove_file(home.join("agent.json"));

    let rt = AgentRuntime::new(&home);
    let st = rt.set_active("openai")?;
    assert_eq!(st.provider, "openai");
    assert!(rt.set_active("no-such-provider").is_err());
    std::fs::remove_dir_all(&home).map_err(|e| e.to_string())
}
