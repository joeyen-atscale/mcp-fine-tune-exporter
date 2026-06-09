// AC1: 5 records, 3 meeting all filter criteria → output has exactly 3 lines, each valid JSON

mod helpers;
use helpers::{good_record, make_store, write_records};
use mcp_fine_tune_exporter::{run_export, ExportConfig, DEFAULT_SYSTEM_PROMPT};
use mcp_trace_store::{BindOutcome, ExecuteOutcome, QualitySignals, TraceRecord};
use serde_json::json;
use tempfile::TempDir;

fn bad_record_no_question(session: &str) -> TraceRecord {
    let mut r = TraceRecord::new(
        session,
        json!({"select": [{"field": "units"}]}),
        BindOutcome::Success,
        ExecuteOutcome::Success { row_count: 5, result_empty: false },
        QualitySignals {
            first_attempt_bind: true,
            bind_attempt_count: 1,
            total_latency_ms: 100,
            tokens_used: None,
        },
    );
    r.grounding_score = Some(0.95);
    // user_question intentionally absent
    r
}

fn bad_record_low_grounding(session: &str) -> TraceRecord {
    let mut r = good_record(session, "what are my sales?", 0.80);
    r
}

#[test]
fn ac1_five_records_three_exported() {
    let tmp = TempDir::new().unwrap();
    let store = make_store(tmp.path());

    let records = vec![
        good_record("s1", "what are total sales?", 0.95),
        good_record("s2", "show me top products", 0.92),
        good_record("s3", "revenue by region", 0.91),
        bad_record_no_question("s4"),
        bad_record_low_grounding("s5"),
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
    assert_eq!(result.exported, 3, "expected 3 exported");

    let content = std::fs::read_to_string(&out).unwrap();
    let lines: Vec<&str> = content.lines().collect();
    assert_eq!(lines.len(), 3, "output must have exactly 3 lines");

    for line in &lines {
        let v: serde_json::Value = serde_json::from_str(line)
            .expect("each line must be valid JSON");
        assert!(v.get("messages").is_some(), "must have messages field");
    }
}
