// AC5: each output record has messages[system, user, assistant]; assistant content is valid JSON

mod helpers;
use helpers::{good_record, make_store, write_records};
use mcp_fine_tune_exporter::{run_export, ExportConfig, DEFAULT_SYSTEM_PROMPT};
use tempfile::TempDir;

#[test]
fn ac5_output_format() {
    let tmp = TempDir::new().unwrap();
    let store = make_store(tmp.path());

    let records = vec![
        good_record("s1", "what are my sales by region?", 0.95),
        good_record("s2", "show top 10 products by revenue", 0.92),
    ];
    write_records(&store, records);

    let out = tmp.path().join("out.jsonl");
    let cfg = ExportConfig {
        store_path: tmp.path().join("trace.jsonl"),
        output: out.clone(),
        min_grounding: 0.90,
        shuffle: false,
        seed: 42,
        max_records: None,
        system_prompt: DEFAULT_SYSTEM_PROMPT.to_string(),
    };

    run_export(&cfg).unwrap();

    let content = std::fs::read_to_string(&out).unwrap();
    for line in content.lines() {
        let v: serde_json::Value = serde_json::from_str(line)
            .expect("line must be valid JSON");

        let messages = v["messages"].as_array().expect("messages must be array");
        assert_eq!(messages.len(), 3, "must have exactly 3 messages");

        assert_eq!(messages[0]["role"], "system");
        assert_eq!(messages[1]["role"], "user");
        assert_eq!(messages[2]["role"], "assistant");

        // assistant content must itself be valid JSON (the MQO)
        let assistant_content = messages[2]["content"].as_str()
            .expect("assistant content must be a string");
        let _: serde_json::Value = serde_json::from_str(assistant_content)
            .expect("assistant content must be valid JSON (the MQO)");
    }
}
