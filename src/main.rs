use std::path::PathBuf;

use clap::Parser;
use serde::Serialize;

use mcp_fine_tune_exporter::{run_export, ExportConfig, FilterReasons, DEFAULT_SYSTEM_PROMPT};

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------

#[derive(Parser, Debug)]
#[command(
    name = "mcp-fine-tune-exporter",
    version,
    about = "Export high-quality MQO interactions as OpenAI chat JSONL for fine-tuning"
)]
struct Args {
    /// Path to the trace store JSONL file (or directory containing it)
    #[arg(long)]
    store: PathBuf,

    /// Output file path
    #[arg(long, default_value = "fine-tune.jsonl")]
    output: PathBuf,

    /// Minimum grounding score threshold (inclusive)
    #[arg(long, default_value_t = 0.90)]
    min_grounding: f64,

    /// Shuffle output order (use --seed for reproducibility)
    #[arg(long)]
    shuffle: bool,

    /// RNG seed for --shuffle
    #[arg(long, default_value_t = 42)]
    seed: u64,

    /// Output format for the summary: human-readable or JSON
    #[arg(long, default_value = "human", value_parser = ["json", "human"])]
    format: String,

    /// Cap the number of exported records
    #[arg(long)]
    max_records: Option<usize>,

    /// Custom system prompt text file
    #[arg(long)]
    system_prompt: Option<PathBuf>,
}

// ---------------------------------------------------------------------------
// Summary types
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct Summary {
    exported: usize,
    filtered_out: usize,
    reasons: FilterReasons,
}

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let system_prompt = if let Some(ref sp_path) = args.system_prompt {
        std::fs::read_to_string(sp_path)?
    } else {
        DEFAULT_SYSTEM_PROMPT.to_string()
    };

    let cfg = ExportConfig {
        store_path: args.store,
        output: args.output.clone(),
        min_grounding: args.min_grounding,
        shuffle: args.shuffle,
        seed: args.seed,
        max_records: args.max_records,
        system_prompt,
    };

    let result = run_export(&cfg)?;

    let summary = Summary {
        exported: result.exported,
        filtered_out: result.filtered_out,
        reasons: result.reasons,
    };

    match args.format.as_str() {
        "json" => {
            println!("{}", serde_json::to_string_pretty(&summary)?);
        }
        _ => {
            eprintln!(
                "Exported {} records to {}. Filtered out: {} (low_grounding={}, retry={}, empty={}, error={}, no_question={})",
                summary.exported,
                args.output.display(),
                summary.filtered_out,
                summary.reasons.low_grounding,
                summary.reasons.retry,
                summary.reasons.empty,
                summary.reasons.error,
                summary.reasons.no_question,
            );
        }
    }

    Ok(())
}
