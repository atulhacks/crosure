use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::{RecorderError, Step};

/// Serializes any JSON value with object keys sorted, so the same step
/// always produces the same bytes.
///
/// ```
/// use serde_json::json;
/// let a = crosure_recorder::canonical_json(&json!({"b": 1, "a": 2}));
/// assert_eq!(a, r#"{"a":2,"b":1}"#);
/// ```
pub fn canonical_json(value: &Value) -> String {
    // serde_json's default `Map` is a BTreeMap, so keys serialize sorted.
    let sorted: Value = sort_keys(value);
    sorted.to_string()
}

fn sort_keys(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut out = serde_json::Map::new();
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            for k in keys {
                if let Some(v) = map.get(k) {
                    out.insert(k.clone(), sort_keys(v));
                }
            }
            Value::Object(out)
        }
        Value::Array(items) => Value::Array(items.iter().map(sort_keys).collect()),
        other => other.clone(),
    }
}

/// The `prev_hash` of the first step of a session.
///
/// ```
/// let g = crosure_recorder::genesis_hash("ses_1");
/// assert!(g.starts_with("sha256:"));
/// assert_eq!(g, crosure_recorder::genesis_hash("ses_1"));
/// ```
pub fn genesis_hash(session_id: &str) -> String {
    hex_digest(format!("crosure:genesis:{session_id}").as_bytes())
}

/// Hash of a step: `sha256(prev_hash || "\n" || canonical_json(step without hash))`.
///
/// ```
/// use crosure_recorder::{step_hash, NewStep, StepKind, Store};
/// let store = Store::open_in_memory()?;
/// let s = store.create_session("t", "/bin/true", "00")?;
/// let step = store.record(&s.id, NewStep::human(StepKind::Load, "crosure"))?;
/// assert_eq!(step_hash(&step)?, step.hash);
/// # Ok::<(), crosure_recorder::RecorderError>(())
/// ```
pub fn step_hash(step: &Step) -> Result<String, RecorderError> {
    let mut value = serde_json::to_value(step)?;
    if let Value::Object(map) = &mut value {
        map.remove("hash");
    }
    let mut bytes = step.prev_hash.as_bytes().to_vec();
    bytes.push(b'\n');
    bytes.extend_from_slice(canonical_json(&value).as_bytes());
    Ok(hex_digest(&bytes))
}

/// `sha256:<hex>` of arbitrary bytes.
pub(crate) fn hex_digest(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let hex: String = digest.iter().map(|b| format!("{b:02x}")).collect();
    format!("sha256:{hex}")
}
