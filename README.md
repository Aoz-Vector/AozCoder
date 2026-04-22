# AozCoder

AozCoder is a standalone ratatui-native CLI consumer for the Vexcoder normalized API. It reads Server-Sent Events, reconstructs canonical `RuntimeEnvelope` values, and renders both interactive and batch surfaces from the same stream.

## Repository Layout

- `src/client/` contains the HTTP transport, SSE parsing, and envelope handling code.
- `src/tui/` contains the event loop and terminal application state.
- `src/ui/` contains the renderers for transcript, prompt, and status surfaces.
- `src/print/` contains the non-interactive output path.
- `schemas/` contains the runtime envelope schema snapshot.
- `docs/` contains the mdBook source.

## Local Build

```sh
cargo build --release
```

## Local Validation

```sh
make gate-fast
```

## Raw GitHub Content Links

```sh
make raw-links
make raw-links RAW_LINKS_ARGS='--branch main'
```

Each line contains the tracked repository path and the corresponding `https://raw.githubusercontent.com/<owner>/<repo>/<branch>/<path>` URL.

## Documentation

```sh
mdbook build
mdbook serve --open
```

The rendered book is written to `book/`.

## License

MIT. See `LICENSE`.