# Getting Started

AozCoder is a terminal UI client that connects to a running Vexcoder server
instance via Server-Sent Events and renders the canonical `RuntimeEnvelope` stream
# Build From Source

AozCoder is a terminal UI client that connects to a running Vexcoder server instance via Server-Sent Events and renders the canonical `RuntimeEnvelope` stream in an interactive ratatui interface.
- A reachable Vexcoder server
## Requirements
- `mdbook` for documentation builds

- Git
rustup toolchain install nightly
cargo install cargo-nextest --locked
- A reachable Vexcoder server
cargo install mdbook --locked
```bash
```

## Build from Source
```sh
git clone https://github.com/Aoz-Vector/AozCoder
cd AozCoder
cargo build --release
```bash

The compiled binary is at `target/release/aozcoder`.

make gate-fast
## Validation

```sh
make gate-fast
## First run

```bash
./target/release/aozcoder --help
./target/release/aozcoder
```

Use `--api-url` and `--api-key` when the endpoint is not a local default.
### Interactive TUI
## Validation
```sh
```bash
make gate-fast
```

The gate runs formatting, clippy, `cargo nextest run`, `cargo test --all-targets`, raw URL sitemap verification, and `mdbook build`.

## Runtime
aozcoder
```bash
aozcoder --api-url https://vexcoder.example.com --api-key $TOKEN
**Key bindings:**

# Pipe into a pager
```

```sh
aozcoder session
## Documentation

The mdBook source is in `docs/`. The generated raw-content index is in `docs/RAW-URL-SITEMAP.md`.
`~/.config/aozcoder/config.toml`).  All keys are optional.

```toml
api_url = "http://localhost:8080"
# api_key = "…"

[ui]
theme = "default"
show_tool_output = true
compact_mode = false

[model]
# default_model = "claude-sonnet-4-5"
# max_tokens = 8096
```

Environment variables override file values.  Nested keys use `__` as a
separator: `AOZCODER_UI__COMPACT_MODE=true`.
