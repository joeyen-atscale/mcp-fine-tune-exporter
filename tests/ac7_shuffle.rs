// AC7: --shuffle with fixed seed produces a different (reproducible) order than input

mod helpers;
use helpers::{good_record, make_store, write_records};
use mcp_fine_tune_exporter::{run_export, ExportConfig, DEFAULT_SYSTEM_PROMPT};
use tempfile::TempDir;

fn extract_record_ids(path: &std::path::Path) -> Vec<String> {
    let content = std::fs::read_to_string(path).unwrap();
    content.lines()
        .map(|line| {
            let v: serde_json::Value = serde_json::from_str(line).unwrap();
            v["metadata"]["record_id"].as_str().unwrap().to_string()
        })
        .collect()
}

#[test]
fn ac7_shuffle_changes_order() {
    let tmp = TempDir::new().unwrap();
    let store = make_store(tmp.path());

    // Need enough records that shuffle will almost certainly change the order
    let records: Vec<_> = (0..20)
        .map(|i| good_record(&format!("s{}", i), &format!("question number {}", i), 0.95))
        .collect();
    write_records(&store, records);

    // Export without shuffle
    let out_ordered = tmp.path().join("ordered.jsonl");
    let cfg_ordered = ExportConfig {
        store_path: tmp.path().join("trace.jsonl"),
        output: out_ordered.clone(),
        min_grounding: 0.90,
        shuffle: false,
        seed: 42,
        max_records: None,
        system_prompt: DEFAULT_SYSTEM_PROMPT.to_string(),
    };
    run_export(&cfg_ordered).unwrap();

    // Export with shuffle (fixed seed)
    let out_shuffled = tmp.path().join("shuffled.jsonl");
    let cfg_shuffled = ExportConfig {
        store_path: tmp.path().join("trace.jsonl"),
        output: out_shuffled.clone(),
        min_grounding: 0.90,
        shuffle: true,
        seed: 42,
        max_records: None,
        system_prompt: DEFAULT_SYSTEM_PROMPT.to_string(),
    };
    run_export(&cfg_shuffled).unwrap();

    let ordered_ids = extract_record_ids(&out_ordered);
    let shuffled_ids = extract_record_ids(&out_shuffled);

    assert_eq!(ordered_ids.len(), shuffled_ids.len(), "same number of records");
    assert_ne!(ordered_ids, shuffled_ids, "shuffle must change order");

    // Same set of IDs, just in different order
    let mut s1 = ordered_ids.clone();
    let mut s2 = shuffled_ids.clone();
    s1.sort();
    s2.sort();
    assert_eq!(s1, s2, "same records, different order");
}

#[test]
fn ac7_shuffle_reproducible_with_seed() {
    let tmp = TempDir::new().unwrap();
    let store = make_store(tmp.path());

    let records: Vec<_> = (0..20)
        .map(|i| good_record(&format!("s{}", i), &format!("question {}", i), 0.95))
        .collect();
    write_records(&store, records);

    let run_shuffled = |out: &std::path::Path, seed: u64| {
        let cfg = ExportConfig {
            store_path: tmp.path().join("trace.jsonl"),
            output: out.to_path_buf(),
            min_grounding: 0.90,
            shuffle: true,
            seed,
            max_records: None,
            system_prompt: DEFAULT_SYSTEM_PROMPT.to_string(),
        };
        run_export(&cfg).unwrap();
    };

    let out1 = tmp.path().join("run1.jsonl");
    let out2 = tmp.path().join("run2.jsonl");
    run_shuffled(&out1, 42);
    run_shuffled(&out2, 42);

    let ids1 = extract_record_ids(&out1);
    let ids2 = extract_record_ids(&out2);
    assert_eq!(ids1, ids2, "same seed must produce same order");
}
