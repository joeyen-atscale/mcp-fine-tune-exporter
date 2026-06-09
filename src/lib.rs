use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

use rand::prelude::SliceRandom;
use rand::SeedableRng;
use serde::Serialize;

use mcp_trace_store::{BindOutcome, ExecuteOutcome, TraceFilter, TraceRecord, TraceStoreConfig, TraceStore};

pub const DEFAULT_SYSTEM_PROMPT: &str =
    "You are an AtScale MCP query assistant. \
     Generate a Multidimensional Query Object (MQO) in JSON format for the given user question.";

// ---------------------------------------------------------------------------
// Output types
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Serialize)]
pub struct FineTuneRecord {
    pub messages: Vec<ChatMessage>,
    pub metadata: RecordMetadata,
}

#[derive(Serialize)]
pub struct RecordMetadata {
    pub record_id: String,
    pub grounding_score: f64,
    pub cluster: Option<String>,
    pub timestamp_ms: u64,
}

#[derive(Debug, Default, Serialize)]
pub struct FilterReasons {
    pub low_grounding: usize,
    pub retry: usize,
    pub empty: usize,
    pub error: usize,
    pub no_question: usize,
}

#[derive(Debug, Serialize)]
pub struct Summary {
    pub exported: usize,
    pub filtered_out: usize,
    pub reasons: FilterReasons,
}

// ---------------------------------------------------------------------------
// Filter logic
// ---------------------------------------------------------------------------

/// Apply fine-tune filter criteria to a single record.
/// Returns Ok(()) if it passes, Err(reason_key) if not.
pub fn passes_filter(record: &TraceRecord, min_grounding: f64) -> Result<(), &'static str> {
    // Must have a user question
    if record.user_question.is_none() {
        return Err("no_question");
    }

    // Grounding score must meet threshold
    let score = record.grounding_score.unwrap_or(0.0);
    if score < min_grounding {
        return Err("low_grounding");
    }

    // Must be first-attempt bind
    if !record.quality.first_attempt_bind {
        return Err("retry");
    }

    // Execute result must be Success with non-empty rows
    match &record.execute_result {
        ExecuteOutcome::Success { row_count, result_empty } => {
            if *result_empty || *row_count == 0 {
                return Err("empty");
            }
        }
        ExecuteOutcome::Error(_) | ExecuteOutcome::Skipped => {
            return Err("error");
        }
    }

    // Bind outcome must be Success
    if record.bind_outcome != BindOutcome::Success {
        return Err("error");
    }

    Ok(())
}

/// Convert a passing TraceRecord into a FineTuneRecord.
pub fn to_fine_tune_record(record: &TraceRecord, system_prompt: &str) -> FineTuneRecord {
    let user_question = record.user_question.as_deref().unwrap_or("").to_string();
    let mqo_str = serde_json::to_string(&record.mqo)
        .unwrap_or_else(|_| record.mqo.to_string());

    FineTuneRecord {
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: system_prompt.to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: user_question,
            },
            ChatMessage {
                role: "assistant".to_string(),
                content: mqo_str,
            },
        ],
        metadata: RecordMetadata {
            record_id: record.record_id.clone(),
            grounding_score: record.grounding_score.unwrap_or(0.0),
            cluster: record.cluster_name.clone(),
            timestamp_ms: record.timestamp_ms,
        },
    }
}

// ---------------------------------------------------------------------------
// Export config and runner
// ---------------------------------------------------------------------------

pub struct ExportConfig {
    pub store_path: PathBuf,
    pub output: PathBuf,
    pub min_grounding: f64,
    pub shuffle: bool,
    pub seed: u64,
    pub max_records: Option<usize>,
    pub system_prompt: String,
}

pub struct ExportResult {
    pub exported: usize,
    pub filtered_out: usize,
    pub reasons: FilterReasons,
}

pub fn run_export(cfg: &ExportConfig) -> anyhow::Result<ExportResult> {
    // Resolve store path: accept either a directory or a direct file path
    let store_file = if cfg.store_path.is_dir() {
        cfg.store_path.join("trace.jsonl")
    } else {
        cfg.store_path.clone()
    };

    let store_cfg = TraceStoreConfig::new(&store_file);
    let store = TraceStore::new(store_cfg)?;

    let filter = TraceFilter::new();
    let all_records = store.scan(&filter)?;

    let total_scanned = all_records.len();

    let mut passing: Vec<&TraceRecord> = Vec::new();
    let mut reasons = FilterReasons::default();

    for rec in &all_records {
        match passes_filter(rec, cfg.min_grounding) {
            Ok(()) => passing.push(rec),
            Err("low_grounding") => reasons.low_grounding += 1,
            Err("retry") => reasons.retry += 1,
            Err("empty") => reasons.empty += 1,
            Err("error") => reasons.error += 1,
            Err("no_question") => reasons.no_question += 1,
            Err(_) => {}
        }
    }

    // Shuffle if requested
    if cfg.shuffle {
        let mut rng = rand::rngs::StdRng::seed_from_u64(cfg.seed);
        passing.shuffle(&mut rng);
    }

    // Cap at max_records
    if let Some(max) = cfg.max_records {
        passing.truncate(max);
    }

    let exported = passing.len();
    let filtered_out = total_scanned - exported;

    // Write output
    let out_file = File::create(&cfg.output)?;
    let mut writer = BufWriter::new(out_file);

    for rec in &passing {
        let ft = to_fine_tune_record(rec, &cfg.system_prompt);
        let line = serde_json::to_string(&ft)?;
        writeln!(writer, "{}", line)?;
    }
    writer.flush()?;

    Ok(ExportResult {
        exported,
        filtered_out,
        reasons,
    })
}
