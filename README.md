# data-pipeline-m3

Run everything from the Nix shell first:

```sh
nix develop
```

Then run the CLI with `cargo run --`:

```sh
cargo run -- --input data/primevul/primevul_valid.jsonl --output /tmp/primevul-enriched.jsonl
```

## Arguments

Required:

- `--input <INPUT>`: PrimeVul JSONL file to read. Use one of the files under `data/primevul/`, or another JSONL file with the same schema.
- `--output <OUTPUT>`: JSONL file to write. Existing files are overwritten unless `--resume` is set.

Optional:

- `--workdir <WORKDIR>`: Directory for cloned repos, cached CPG files, and Joern output. Default: `.primevul-work`. Use a persistent directory if you want reruns to reuse cached work.
- `--joern <JOERN>`: Joern shell binary. Default: `joern`. Keep the default inside `nix develop` unless you need a custom Joern install.
- `--joern-parse <JOERN_PARSE>`: `joern-parse` binary. Default: `joern-parse`. Keep the default inside `nix develop` unless you need a custom Joern install.
- `--joern-script <JOERN_SCRIPT>`: Scala script used for extraction. Default: `joern/extract_method_summary.sc`.
- `--resume`: Append to the output file and skip rows whose `id` is already present there. Use this when continuing an interrupted run.
- `--limit <LIMIT>`: Process at most this many input rows. Default: `0`, meaning no limit. Use small values like `1` or `10` for testing.
- `--skip-joern`: Skip git checkout and Joern. Use this to test JSON parsing and prompt/output generation quickly.

Full shape:

```sh
cargo run -- \
  --input data/primevul/primevul_train.jsonl \
  --output /tmp/primevul-train-enriched.jsonl \
  --workdir .primevul-work \
  --limit 100 \
  --resume
```

Get the generated help:

```sh
cargo run -- --help
```
