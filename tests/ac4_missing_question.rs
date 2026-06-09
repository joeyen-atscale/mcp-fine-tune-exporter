// AC4: record missing user_question is excluded and counted in no_question

mod helpers;
use helpers::{good_record, make_store, write_records};
use mcp_fine_tune_exporter::{run_export, ExportConfig, DEFAULT_SYSTEM_PROMPT};
use mcp_trace_store::{BindOutcome, ExecuteOutcome, QualitySignals, TraceRecord};
use serde_json::json;
use tempfile::TempDir;

fn no_question_record(session: &str) -> TraceRecord {
    let mut r = TraceRecord::new(
        session,
        json!({"select": [{"field": "count"}]}),
        BindOutcome::Success,
        ExecuteOutcome::Success { row_count: 7, result_empty: false },
        QualitySignals {
            first_attempt_bind: true,
            bind_attempt_count: 1,
            total_latency_ms: 150,
            tokens_used: None,
        },
    );
    r.grounding_score = Some(0.93);
    // user_question is None (legacy record)
    r
}

#[test]
fn ac4_no_question_excluded() {
    let tmp = TempDir::new().unwrap();
    let store = make_store(tmp.path());

    let records = vec![
        good_record("s1", "a good question", 0.95),
        no_question_record("s2"),
        no_question_record("s3"),
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
    assert_eq!(result.exported, 1);
    assert_eq!(result.reasons.no_question, 2, "two no_question exclusions");
}
