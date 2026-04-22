ifeq ($(OS),Windows_NT)
SHELL := C:/Program Files/Git/bin/bash.exe
else
SHELL := bash
endif
.SHELLFLAGS := -euo pipefail -c

BRANCH ?= $(shell branch=$$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo main); if [ "$$branch" = "HEAD" ]; then echo main; else echo $$branch; fi)

.PHONY: help _require-nextest _require-mdbook build check fmt fmt-check lint test test-nextest docs docs-check raw-links gate gate-fast clean

help:
	@printf '%s\n' \
	  "Targets:" \
	  "  build       cargo build --all-targets" \
	  "  check       cargo check --all-targets" \
	  "  fmt         cargo fmt --all" \
	  "  fmt-check   cargo fmt --all --check" \
	  "  lint        cargo clippy --all-targets --all-features -- -D warnings" \
	  "  test        cargo test --all-targets" \
	  "  test-nextest cargo nextest run --all-features" \
	  "  docs        mdbook build" \
	  "  raw-links   cargo run --bin raw-links -- --branch $(BRANCH)" \
	  "  gate        fmt-check + lint + nextest + all-target tests + docs" \
	  "  gate-fast   identical to gate" \
	  "  clean       cargo clean"

_require-nextest:
	@command -v cargo-nextest >/dev/null 2>&1 || { \
	  echo "MISSING TOOL: cargo nextest run"; \
	  echo "  Install: cargo install cargo-nextest --locked"; \
	  exit 1; \
	}

_require-mdbook:
	@command -v mdbook >/dev/null 2>&1 || { \
	  echo "MISSING TOOL: mdbook"; \
	  echo "  Install: cargo install mdbook --locked"; \
	  exit 1; \
	}

build:
	cargo build --all-targets

check:
	cargo check --all-targets

fmt:
	cargo fmt --all

fmt-check:
	cargo fmt --all --check

lint:
	cargo clippy --all-targets --all-features -- -D warnings

test:
	cargo test --all-targets

test-nextest: _require-nextest
	cargo nextest run --all-features

docs: _require-mdbook
	mdbook build

docs-check: docs

raw-links:
	cargo run --quiet --bin raw-links -- --branch "$(BRANCH)" $(RAW_LINKS_ARGS)

gate: fmt-check lint test-nextest test docs-check

gate-fast: gate

clean:
	cargo clean