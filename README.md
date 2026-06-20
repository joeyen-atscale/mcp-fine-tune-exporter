# mcp-fine-tune-exporter

Turn the AtScale MCP trace store into a fine-tuning dataset: filter for the interactions that actually went well, and emit them as OpenAI chat JSONL.

## Why it exists

Every MQO interaction the MCP server handles is already recorded in the trace store — the user's question, the query object the model produced, whether it bound on the first try, whether it executed, how well it was grounded. That history is the best training data there is for teaching a model to write a Multidimensional Query Object: real questions, real answers, and a record of which answers were good.

The problem is that most of it isn't good. Retries, empty results, low-grounding guesses, records with no question attached — a raw trace dump would teach a model the failures alongside the successes. This tool keeps only the records that meet every quality bar, and writes them in the shape a fine-tuning API expects.

## What it produces

One JSONL line per passing record. Each line is a chat example plus the metadata used to select it:

```json
{
  "messages": [
    {"role": "system", "content": "You are an AtScale MCP query assistant. Generate a Multidimensional Query Object (MQO) in JSON format for the given user question."},
    {"role": "user", "content": "what are my sales by region?"},
    {"role": "assistant", "content": "{\"select\":[{\"field\":\"sales\"}],\"filter\":[]}"}
  ],
  "metadata": {
    "record_id": "...",
    "grounding_score": 0.95,
    "cluster": "...",
    "timestamp_ms": 1718000000000
  }
}
```

The assistant message is the MQO serialized as a JSON string, so the whole line is itself valid JSON. The `metadata` block records why the record qualified; strip it before upload if your fine-tuning API rejects extra keys.

## Install

The crate depends on `mcp-trace-store` as a path dependency, so build it from a checkout that has `mcp-trace-store` as a sibling directory:

```bash
cargo build --release
cp target/release/mcp-fine-tune-exporter ~/.local/bin/
```

## Quickstart

```bash
mcp-fine-tune-exporter \
  --store ~/.local/share/mcp-traces/trace.jsonl \
  --output finetune-dataset.jsonl
```

`--store` takes either the JSONL file directly or a directory containing `trace.jsonl`. The exporter scans every record, keeps the ones that pass all filters, and writes them to `--output`. A summary lands on stderr:

```
Exported 4217 records to finetune-dataset.jsonl. Filtered out: 45783 (low_grounding=30000, retry=10000, empty=3000, error=2000, no_question=783)
```

Pass `--format json` to get that summary as JSON on stdout instead, which is what you want when a script consumes the result:

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

## What passes the filter

A record is exported only if all of these hold:

1. A `user_question` is present. Records without one are dropped (`no_question`).
2. `grounding_score` is at least `--min-grounding`, default `0.90` (`low_grounding`).
3. The bind succeeded on the first attempt — `quality.first_attempt_bind` is true, no retries (`retry`).
4. The query executed as `Success` with `row_count > 0` and `result_empty == false` (`empty`).
5. The bind outcome is `Success` (`error`; an execute `Error` or `Skipped` also lands here).

The summary counts each dropped record under the first reason it failed.

## Options

| Flag | Default | What it does |
|------|---------|--------------|
| `--store` | required | Trace JSONL file, or a directory containing `trace.jsonl` |
| `--output` | `fine-tune.jsonl` | Output JSONL path |
| `--min-grounding` | `0.90` | Minimum grounding score, inclusive |
| `--max-records` | unlimited | Cap the number of exported records |
| `--shuffle` | off | Fisher-Yates shuffle of the output order |
| `--seed` | `42` | RNG seed, so a shuffle is reproducible |
| `--system-prompt` | built-in | Path to a text file overriding the default system prompt |
| `--format` | `human` | `human` (summary on stderr) or `json` (summary on stdout) |

`--max-records` truncates after the shuffle, so combining `--shuffle` with `--max-records` gives a reproducible random sample.

## Privacy

This version does not redact PII. User questions and MQO content go into the output verbatim, which can include customer names, internal hostnames, or proprietary metric names. Review the JSONL before sending it to any external fine-tuning API. A `--redact` flag is not yet implemented.

## Where it fits

Part of the MQO/MCP toolchain. It reads the records written by `mcp-trace-store`; the dataset it produces trains a model to write the same MQOs the MCP server already serves.

## Status

v0.1.0. The filter pipeline and JSONL output are covered by acceptance tests (`tests/ac1`–`ac7`). Redaction is the known gap.
