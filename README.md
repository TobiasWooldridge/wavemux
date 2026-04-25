# wavemux

Multiplexed audio-stream protocol for SDR applications. Packs audio
from many substreams (talkgroups, channels) into a single connection,
with silent substreams emitting nothing.

## Two wire encodings

- **Binary** (`src/wire.rs`) — 12-byte subframe header + payload,
  multiple subframes per frame. The format is documented in the
  module docstring at the top of `src/lib.rs`.
- **JSONL** (`src/jsonl.rs`) — NDJSON for HTTP consumers; each line
  is a self-contained `{"type": …}` object.

Subframe types: `Audio`, `CallStart`, `CallEnd`, `StreamInfo`.
Codecs: `Pcm16Le`, `Pcm16Le8k`, `Opus`, `ImbeRaw`.

## Why this is a separate crate

`wavemux` has its own version cadence and dependency set (just
`serde` + `base64`), so it lives as its own repo + nested submodule
and is **deliberately excluded from the WaveCatch workspace** (see
"Hard rules" in `../CLAUDE.md`). Producers and consumers each pin
it independently.

## Producers and consumers

- **Producer:** `wsrc-server`'s `wavesource_trunked` audio mux
  endpoints (HTTP + raw TCP).
- **Consumer:** `wavecap`'s `wcap-ingest::wavemux_jsonl` parser. As
  of writing, decode of the audio payload is raw-PCM only — Opus
  and IMBE decoding are tracked in wavecap#1.

When you change a wire field here, bump the wavemux submodule
pointer in WaveCatch and check the wavecap parser.
