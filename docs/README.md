# wavemux Docs

Protocol reference material for the shared multiplexed audio stream crate.
Implementation source remains in the Rust modules; this directory is for
reviewable prose.

## Contents

- [`protocol.md`](protocol.md) - binary subframe and JSONL protocol reference
  for `wavemux` producers and consumers.

## Read First

- Start with [`../README.md`](../README.md) for crate scope and the high-level
  protocol shape.
- Use [`protocol.md`](protocol.md) for the reviewable wire-format reference.
- Check [`../src/wire.rs`](../src/wire.rs) and [`../src/jsonl.rs`](../src/jsonl.rs)
  before changing protocol details; those modules are the executable source of
  truth.

## Working Rules

- Keep protocol facts aligned with the Rust encode/decode modules.
- Link new protocol notes from this README.
- Validate docs with `python3 scripts/check-md-links.py --all`.
