# mcp-fine-tune-exporter

Export high-quality MQO interactions from the AtScale MCP trace store as LLM fine-tuning data in OpenAI chat JSONL format.

## What it does

Reads a `mcp-trace-store` JSONL file, applies quality filters, and writes one fine-tuning example per passing record:

```json
{"messages": [
  {"role": "system", "content": "You are an AtScale MCP query assistant. Generate a Multidimensional Query Object (MQO) in JSON format for the given user question."},
  {"role": "user", "content": "<user_question>"},
  {"role": "assistant", "content": "<mqo serialized as JSON string>"}
]}
```

## Filter criteria (all must hold)

1. `grounding_score >= --min-grounding` (default 0.90)
2. `quality.first_attempt_bind == true` (no retries)
3. `execute_result` is `Success` with `row_count > 0` and `result_empty == false`
4. `bind_outcome == Success`
5. `user_question` is present (records without it are excluded)

## Usage

```
mcp-fine-tune-exporter \
  --store ~/.local/share/mcp-traces/trace.jsonl \
  --output finetune-dataset.jsonl \
  [--min-grounding 0.90] \
  [--max-records 5000] \
  [--shuffle] \
  [--seed 42] \
  [--system-prompt system_prompt.txt] \
  [--format json|human]
```

### Options

| Flag | Default | Description |
|------|---------|-------------|
| `--store` | required | Path to the trace JSONL file or directory containing `trace.jsonl` |
| `--output` | `fine-tune.jsonl` | Output JSONL file path |
| `--min-grounding` | `0.90` | Minimum grounding score (inclusive) |
| `--max-records` | unlimited | Cap number of output records |
| `--shuffle` | off | Fisher-Yates shuffle output order |
| `--seed` | `42` | RNG seed for reproducible shuffle |
| `--system-prompt` | built-in | Path to a text file with a custom system prompt |
| `--format` | `human` | Summary output: `human` (stderr) or `json` (stdout) |

### Summary output

Human format (stderr):
```
Exported 4217 records to finetune-dataset.jsonl. Filtered out: 45783 (low_grounding=30000, retry=10000, empty=3000, error=2000, no_question=783)
```

JSON format (stdout):
```json
{
  "exported": 4217,
  "filtered_out": 45783,
  "reasons": {
    "low_grounding": 30000,
    "retry": 10000,
    "empty": 3000,
    "error": 2000,
    "no_question": 783
  }
}
```

## Installation

```bash
cargo build --release
cp target/release/mcp-fine-tune-exporter ~/.local/bin/
```

## Privacy note

v1 does **not** redact PII from user questions or MQO content. Before sharing exported JSONL with any external fine-tuning API, review records for sensitive data (customer names, internal hostnames, proprietary metric names). A future version will add `--redact` support.

## Dependencies

- `serde` / `serde_json` — serialization
- `clap` — CLI argument parsing
- `rand` — Fisher-Yates shuffle
- `anyhow` — error handling
- `mcp-trace-store` (path dep) — trace record types and store reader
