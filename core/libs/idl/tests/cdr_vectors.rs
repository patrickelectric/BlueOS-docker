//! The shared CDR test vectors: every vector decodes to its `decoded` value and, unless marked otherwise, encodes
//! back to the same bytes.

use blueos_idl::{Message, blueos_example_msgs, blueos_msgs, cdr, foxglove_msgs};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;

const PADDING: &str = "structure_needs_at_least_one_member";

fn check<M: Message + Serialize + DeserializeOwned>(vector: &Value) -> Result<(), String> {
    let hex = vector["hex"].as_str().ok_or("hex is not a string")?;
    let bytes: Vec<u8> = (0..hex.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&hex[index..index + 2], 16))
        .collect::<Result<_, _>>()
        .map_err(|error| error.to_string())?;

    let decoded: M = cdr::decode(&bytes).map_err(|error| error.to_string())?;
    let mut decoded = serde_json::to_value(&decoded).map_err(|error| error.to_string())?;
    if let Some(object) = decoded.as_object_mut() {
        object.remove(PADDING);
    }
    if decoded != vector["decoded"] {
        return Err(format!("decoded {decoded} instead of {}", vector["decoded"]));
    }

    if vector["skip_encode_round_trip"] != Value::Bool(true) {
        let mut expected = vector["decoded"].clone();
        if expected.as_object().is_some_and(serde_json::Map::is_empty) {
            expected[PADDING] = 0.into();
        }
        let message: M = serde_json::from_value(expected).map_err(|error| error.to_string())?;
        let encoded = cdr::encode(&message).map_err(|error| error.to_string())?;
        if encoded != bytes {
            return Err(format!("encoded {encoded:02x?}"));
        }
    }
    Ok(())
}

type Check = fn(&Value) -> Result<(), String>;

fn entry<M: Message + Serialize + DeserializeOwned>() -> (&'static str, Check) {
    (M::SCHEMA_NAME, check::<M>)
}

#[test]
fn every_vector_round_trips() {
    let checks = [
        entry::<blueos_example_msgs::PumpState>(),
        entry::<blueos_example_msgs::SetLevelGoal>(),
        entry::<blueos_example_msgs::SetLevelResult>(),
        entry::<blueos_example_msgs::SetLevelFeedback>(),
        entry::<blueos_example_msgs::LevelRequest>(),
        entry::<blueos_example_msgs::LevelResponse>(),
        entry::<blueos_msgs::CommandAck>(),
        entry::<blueos_msgs::EndpointInfo>(),
        entry::<blueos_msgs::JobFeedback>(),
        entry::<blueos_msgs::JobFeedbackList>(),
        entry::<blueos_msgs::JobList>(),
        entry::<blueos_msgs::JobResult>(),
        entry::<blueos_msgs::JobStatus>(),
        entry::<blueos_msgs::ServiceInfo>(),
        entry::<blueos_msgs::SettingField>(),
        entry::<blueos_msgs::SettingsEnvelope>(),
        entry::<blueos_msgs::UpdateSettingsGoal>(),
        entry::<blueos_msgs::UpdateSettingsResult>(),
        entry::<blueos_msgs::UpdateSettingsFeedback>(),
        entry::<foxglove_msgs::Log>(),
    ];
    let vectors: Value = serde_json::from_str(include_str!("vectors/cdr.json")).unwrap();
    let mut failures = Vec::new();
    for vector in vectors["vectors"].as_array().unwrap() {
        let schema_name = vector["schema_name"].as_str().unwrap();
        let category = vector["category"].as_str().unwrap_or_default();
        match checks.iter().find(|(name, _)| *name == schema_name) {
            Some((_, check)) => {
                if let Err(error) = check(vector) {
                    failures.push(format!("{schema_name} ({category}): {error}"));
                }
            }
            None => failures.push(format!("{schema_name}: no generated type")),
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
