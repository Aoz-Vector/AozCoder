# AozCoder

AozCoder is a standalone ratatui-native CLI consumer for the Vexcoder normalized API.

## Build from source

Requires Git, a nightly Rust toolchain with `cargo` on `PATH`, `cargo-nextest`, `mdbook`, write access in the checkout, and a reachable Vexcoder endpoint.

```bash
git clone https://github.com/Aoz-Vector/AozCoder.git
cd AozCoder
cargo build --release
make gate-fast
./target/release/aozcoder --help
```

The built binary is at `target/release/aozcoder`.

Local and private-network endpoints can stay on plain HTTP. Public remote endpoints should use `https://` and an API token.

## Documentation

Full documentation is in `docs/`.

- [Build From Source](docs/GETTING-STARTED.md)
- [Raw URL Sitemap](docs/RAW-URL-SITEMAP.md)

To read the book locally:

```bash
mdbook serve --open
```

## Generated files

`docs/RAW-URL-SITEMAP.md` is generated from `git ls-files` and verified by `make gate-fast` on every push through the `ci` workflow.

## Repository layout

- `src/client/` contains the HTTP transport, SSE parsing, and envelope handling code.
- `src/tui/` contains the terminal event loop and application state.
- `src/ui/` contains the renderers for transcript, prompt, and status surfaces.
- `src/print/` contains the non-interactive output path.
- `schemas/` contains the runtime envelope schema snapshot.

MIT. See `LICENSE`.