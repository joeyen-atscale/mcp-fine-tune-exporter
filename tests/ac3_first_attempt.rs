// AC3: record with first_attempt_bind=false is excluded even if grounding+result are perfect

mod helpers;
use helpers::{good_record, make_store, write_records};
use mcp_fine_tune_exporter::{run_export, ExportConfig, DEFAULT_SYSTEM_PROMPT};
use mcp_trace_store::{BindOutcome, ExecuteOutcome, QualitySignals, TraceRecord};
use serde_json::json;
use tempfile::TempDir;

fn retry_record(session: &str) -> TraceRecord {
    let mut r = TraceRecord::new(
        session,
        json!({"select": [{"field": "revenue"}]}),
        BindOutcome::Success,
        ExecuteOutcome::Success { row_count: 20, result_empty: false },
        QualitySignals {
            first_attempt_bind: false, // retried
            bind_attempt_count: 3,
            total_latency_ms: 500,
            tokens_used: None,
        },
    );
    r.grounding_score = Some(0.99);
    r.user_question = Some("what is total revenue?".to_string());
    r
}

#[test]
fn ac3_retry_excluded() {
    let tmp = TempDir::new().unwrap();
    let store = make_store(tmp.path());

    let records = vec![
        good_record("s1", "first attempt question", 0.95),
        retry_record("s2"),
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
    assert_eq!(result.exported, 1, "retry record should be excluded");
    assert_eq!(result.reasons.retry, 1, "retry counter must be 1");
}
