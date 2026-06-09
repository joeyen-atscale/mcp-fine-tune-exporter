// AC2: grounding_score=0.85 is excluded; grounding_score=0.90 is included (at threshold)

mod helpers;
use helpers::{good_record, make_store, write_records};
use mcp_fine_tune_exporter::{run_export, ExportConfig, DEFAULT_SYSTEM_PROMPT};
use tempfile::TempDir;

#[test]
fn ac2_threshold_boundary() {
    let tmp = TempDir::new().unwrap();
    let store = make_store(tmp.path());

    let records = vec![
        good_record("s1", "at threshold question", 0.90),   // included
        good_record("s2", "below threshold question", 0.85), // excluded
        good_record("s3", "above threshold question", 0.95), // included
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

    let result = run_export(&cfg).unwrap();
    assert_eq!(result.exported, 2, "0.90 and 0.95 should pass; 0.85 should not");
    assert_eq!(result.reasons.low_grounding, 1, "exactly one low-grounding exclusion");

    let content = std::fs::read_to_string(&out).unwrap();
    assert_eq!(content.lines().count(), 2);
}
