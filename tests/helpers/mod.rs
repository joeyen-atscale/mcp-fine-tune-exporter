use mcp_trace_store::{BindOutcome, ExecuteOutcome, QualitySignals, TraceRecord, TraceStoreConfig, TraceStore};
use serde_json::json;

pub fn make_store(dir: &std::path::Path) -> TraceStore {
    let store_file = dir.join("trace.jsonl");
    let cfg = TraceStoreConfig::new(&store_file);
    TraceStore::new(cfg).expect("create store")
}

pub fn good_record(session: &str, question: &str, grounding: f64) -> TraceRecord {
    let mut r = TraceRecord::new(
        session,
        json!({"select": [{"field": "sales"}], "filter": []}),
        BindOutcome::Success,
        ExecuteOutcome::Success { row_count: 10, result_empty: false },
        QualitySignals {
            first_attempt_bind: true,
            bind_attempt_count: 1,
            total_latency_ms: 200,
            tokens_used: None,
        },
    );
    r.grounding_score = Some(grounding);
    r.user_question = Some(question.to_string());
    r
}

pub fn write_records(store: &TraceStore, records: Vec<TraceRecord>) {
    for r in records {
        store.append(r).expect("append record");
    }
}
