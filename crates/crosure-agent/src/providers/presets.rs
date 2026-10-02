use super::config::{ProviderConfig, ProviderKind};

use ProviderKind::{Anthropic, OpenaiCompatible as Oai};

/// `(id, kind, label, base URL, model, key env var, strict tools)`.
type Row = (
    &'static str,
    ProviderKind,
    &'static str,
    &'static str,
    &'static str,
    Option<&'static str>,
    bool,
);

#[rustfmt::skip]
const ROWS: &[Row] = &[
    ("anthropic", Anthropic, "Anthropic Claude", "https://api.anthropic.com", super::DEFAULT_MODEL, Some("ANTHROPIC_API_KEY"), true),
    ("openai", Oai, "OpenAI", "https://api.openai.com/v1", "", Some("OPENAI_API_KEY"), true),
    ("gemini", Oai, "Google Gemini", "https://generativelanguage.googleapis.com/v1beta/openai", "", Some("GEMINI_API_KEY"), false),
    ("deepseek", Oai, "DeepSeek", "https://api.deepseek.com/v1", "", Some("DEEPSEEK_API_KEY"), false),
    ("zai", Oai, "Z.ai (GLM)", "https://api.z.ai/api/paas/v4", "", Some("ZAI_API_KEY"), false),
    ("moonshot", Oai, "Moonshot (Kimi)", "https://api.moonshot.ai/v1", "", Some("MOONSHOT_API_KEY"), false),
    ("xai", Oai, "xAI (Grok)", "https://api.x.ai/v1", "", Some("XAI_API_KEY"), false),
    ("qwen", Oai, "Alibaba Qwen (DashScope)", "https://dashscope-intl.aliyuncs.com/compatible-mode/v1", "", Some("DASHSCOPE_API_KEY"), false),
    ("mistral", Oai, "Mistral", "https://api.mistral.ai/v1", "", Some("MISTRAL_API_KEY"), false),
    ("openrouter", Oai, "OpenRouter", "https://openrouter.ai/api/v1", "", Some("OPENROUTER_API_KEY"), false),
    ("groq", Oai, "Groq", "https://api.groq.com/openai/v1", "", Some("GROQ_API_KEY"), false),
    ("together", Oai, "Together AI", "https://api.together.xyz/v1", "", Some("TOGETHER_API_KEY"), false),
    ("fireworks", Oai, "Fireworks AI", "https://api.fireworks.ai/inference/v1", "", Some("FIREWORKS_API_KEY"), false),
    ("cerebras", Oai, "Cerebras", "https://api.cerebras.ai/v1", "", Some("CEREBRAS_API_KEY"), false),
    ("ollama", Oai, "Ollama (local)", "http://localhost:11434/v1", "", None, false),
    ("lmstudio", Oai, "LM Studio (local)", "http://localhost:1234/v1", "", None, false),
    ("llamacpp", Oai, "llama.cpp server (local)", "http://localhost:8080/v1", "", None, false),
    ("vllm", Oai, "vLLM (local)", "http://localhost:8000/v1", "", None, false),
    ("custom", Oai, "Custom (OpenAI-compatible)", "http://localhost:8000/v1", "", None, false),
    ("custom-anthropic", Anthropic, "Custom (Anthropic-compatible)", "", "", None, false),
];

/// Ready-made provider templates. Models other than Claude are left for the
/// user to pick (use "Fetch models"), so nothing here goes stale.
///
/// ```
/// let p = crosure_agent::presets();
/// assert!(p.iter().any(|p| p.id == "ollama" && p.base_url.contains("11434")));
/// assert!(p.iter().any(|p| p.id == "zai" && p.key_env.as_deref() == Some("ZAI_API_KEY")));
/// ```
pub fn presets() -> Vec<ProviderConfig> {
    ROWS.iter()
        .map(
            |&(id, kind, label, base, model, env, strict)| ProviderConfig {
                id: id.into(),
                kind,
                label: label.into(),
                base_url: base.into(),
                model: model.into(),
                api_key: None,
                key_env: env.map(str::to_string),
                strict_tools: strict,
                enabled: true,
            },
        )
        .collect()
}
