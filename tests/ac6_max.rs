// AC6: --max-records caps output even when more records pass the filter

mod helpers;
use helpers::{good_record, make_store, write_records};
use mcp_fine_tune_exporter::{run_export, ExportConfig, DEFAULT_SYSTEM_PROMPT};
use tempfile::TempDir;

#[test]
fn ac6_max_records_cap() {
    let tmp = TempDir::new().unwrap();
    let store = make_store(tmp.path());

    // 10 records all passing
    let records: Vec<_> = (0..10)
        .map(|i| good_record(&format!("s{}", i), &format!("question {}", i), 0.95))
        .collect();
    write_records(&store, records);

    let out = tmp.path().join("out.jsonl");
    let cfg = ExportConfig {
        store_path: tmp.path().join("trace.jsonl"),
        output: out.clone(),
        min_grounding: 0.90,
        shuffle: false,
        seed: 42,
        max_records: Some(3),
        system_prompt: DEFAULT_SYSTEM_PROMPT.to_string(),
    };

    let result = run_export(&cfg).unwrap();
    assert_eq!(result.exported, 3, "must cap at max_records=3");

    let content = std::fs::read_to_string(&out).unwrap();
    assert_eq!(content.lines().count(), 3);
}
