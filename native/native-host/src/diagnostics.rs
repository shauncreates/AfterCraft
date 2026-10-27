//! Summarize observed ABI failures without guessing vendor error or licensing meanings.
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

pub fn summarize(events: &[Value]) -> Value {
    let mut phase = "initialization";
    let mut missing = BTreeMap::<(String, i64), (usize, BTreeSet<String>)>::new();
    let mut callbacks = Vec::new();
    let mut messages = Vec::new();
    let mut exceptions = Vec::new();
    for event in events {
        let stage = event["stage"].as_str().unwrap_or("");
        if matches!(
            stage,
            "metadata"
                | "load"
                | "global_setup"
                | "params_setup"
                | "sequence_setup"
                | "frame_setup"
                | "render"
                | "smart_pre_render"
                | "smart_render"
                | "frame_setdown"
                | "sequence_setdown"
                | "global_setdown"
        ) {
            phase = stage;
        }
        if stage == "suite_requested"
            && event["supported"] == false
            && let (Some(name), Some(version)) = (event["name"].as_str(), event["version"].as_i64())
        {
            let (count, phases) = missing.entry((name.to_owned(), version)).or_default();
            *count += 1;
            phases.insert(phase.to_owned());
        }
        if matches!(stage, "unsupported_callback" | "unsupported_checkout") {
            callbacks.push(json!({"phase":phase,"event":event}));
        }
        if let Some(message) = event["message"].as_str().filter(|s| !s.is_empty()) {
            messages.push(json!({"stage":stage,"message":message}));
        }
        if stage == "native_exception" {
            exceptions.push(json!({"phase":phase,"event":event}));
        }
    }
    let missing: Vec<_> =
        missing.into_iter().map(|((name, version), (count, phases))| json!({"name":name,"version":version,"requests":count,"phases":phases})).collect();
    json!({"missingSuites":missing,"unsupportedCallbacks":callbacks,
        "pluginMessages":messages,"nativeExceptions":exceptions,
        "lastPhase":phase,
        "interpretation":"Observed calls only. Missing suites may be optional. A successful return or written frame does not establish correct output or licensing."})
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn groups_missing_versions_and_preserves_failure_phase() {
        let result = summarize(&[
            json!({"stage":"global_setup"}),
            json!({"stage":"suite_requested","name":"Layer","version":1,"supported":false}),
            json!({"stage":"render"}),
            json!({"stage":"suite_requested","name":"Layer","version":1,"supported":false}),
            json!({"stage":"suite_requested","name":"Layer","version":2,"supported":true}),
            json!({"stage":"suite_requested","supported":false}),
            json!({"stage":"native_exception","code":123}),
            json!({"stage":"render_result","code":1,"message":"Original vendor message"}),
        ]);
        assert_eq!(result["missingSuites"].as_array().unwrap().len(), 1);
        assert_eq!(result["missingSuites"][0]["requests"], 2);
        assert_eq!(result["missingSuites"][0]["phases"], json!(["global_setup", "render"]));
        assert_eq!(result["nativeExceptions"][0]["phase"], "render");
        assert_eq!(result["pluginMessages"][0]["message"], "Original vendor message");
    }
}
