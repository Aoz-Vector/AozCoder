# Getting Started

AozCoder is a terminal UI client that connects to a running Vexcoder server
instance via Server-Sent Events and renders the canonical `RuntimeEnvelope` stream
in an interactive ratatui interface.

## Prerequisites

- Rust nightly (see `rust-toolchain.toml`)
- A reachable Vexcoder server
- `cargo-nextest` for the full local gate
- `mdbook` for documentation builds

```
rustup toolchain install nightly
cargo install cargo-nextest --locked
cargo install mdbook --locked
rustup show  # confirms the toolchain file takes effect
```

## Build from Source

```sh
git clone https://github.com/Aoz-Vector/AozCoder
cd AozCoder
cargo build --release
```

The compiled binary is at `target/release/aozcoder`.

## Validation

```sh
make gate-fast
```

The gate runs formatting, clippy, `cargo nextest run`, `cargo test --all-targets`,
and `mdbook build`.

## Usage

### Interactive TUI

```sh
# Connect to a local server (default: http://localhost:8080)
aozcoder

# Connect to a remote server with an API key
aozcoder --api-url https://vexcoder.example.com --api-key $TOKEN
```

**Key bindings:**

| Key | Action |
|-----|--------|
| `Enter` | Submit prompt |
| `Ctrl-C` | Interrupt streaming turn |
| `Ctrl-Q` | Quit |
| `Esc` | Clear prompt / dismiss error |
| `↑ / ↓` | Scroll transcript |
| `End` | Resume auto-scroll |

### Batch Mode

```sh
# Print response as text (default)
aozcoder run "Explain the Liskov substitution principle"

# JSON-encoded string output
aozcoder run --format json "What is RFC 9113?"

# Pipe into a pager
aozcoder run "List Rust 2024 edition changes" | less
```

### Session Inspection

```sh
aozcoder session
```

## Configuration

AozCoder reads `$XDG_CONFIG_HOME/aozcoder/config.toml` (falling back to
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

## Running Tests

```sh
# All tests via cargo-nextest
cargo nextest run

# Integration tests only (requires network access to spawn a mock server)
cargo test --test integration
```

## Raw Repository Links

```sh
make raw-links
make raw-links RAW_LINKS_ARGS='--branch main'
```

The utility enumerates tracked files with `git ls-files` and prints one
`raw.githubusercontent.com` URL per file.
