# wavemux

`wavemux` is the shared multiplexed audio stream protocol crate for the SDR
stack. It lets a producer carry many radio substreams, such as conventional
channels or trunked talkgroups, over one connection while omitting silent
substreams entirely.

The main producer is
[`WaveCatch`](https://github.com/TobiasWooldridge/WaveCatch), and the main
consumer today is [`WaveCap`](https://github.com/TobiasWooldridge/WaveCap).
Keep this crate small and protocol-focused so both sides can update the wire
boundary without depending on each other's full workspaces.

## Read First

- [`src/lib.rs`](src/lib.rs) for the public re-exports and crate-level format
  summary.
- [`docs/protocol.md`](docs/protocol.md) for the binary and JSONL protocol
  reference.
- [`src/wire.rs`](src/wire.rs) for binary subframe headers, codecs, frame
  types, and encode/decode helpers.
- [`src/jsonl.rs`](src/jsonl.rs) for newline-delimited JSON used by HTTP
  consumers.

## Protocol Shape

Binary mux payloads are a concatenation of subframes. Each subframe starts
with a 12-byte big-endian header:

| Offset | Size | Field |
| --- | --- | --- |
| 0 | 2 | `substream_id` |
| 2 | 1 | `subframe_type` |
| 3 | 1 | `codec` |
| 4 | 4 | `source_id` |
| 8 | 4 | `payload_len` |

JSONL presents the same subframes as one JSON object per line. Audio payloads
use base64 in `samples_b64`; control frames carry JSON metadata in `data`.

## Frame Types

| Type | Purpose |
| --- | --- |
| `audio` | PCM, Opus, or raw IMBE audio payload for a substream. |
| `call_start` | Start metadata for a transmission or trunked call. |
| `call_end` | End metadata for a transmission or trunked call. |
| `stream_info` | Substream metadata sent on subscribe or metadata change. |
| `call_metadata_update` | Mid-call metadata diff, such as encryption state. |
| `location` | Unit location report metadata. |

## Codecs

| Codec | Meaning |
| --- | --- |
| `pcm16le` | Mono i16 little-endian at 48 kHz. |
| `pcm16le_8k` | Mono i16 little-endian at 8 kHz. |
| `opus` | Single Opus frame. |
| `imbe_raw` | Raw 88-bit IMBE voice frame. |

## Validation

Run the crate tests before changing protocol behavior:

```bash
cargo test
```

From the parent SDR checkout, the doc-hygiene checks can also validate this
README:

```bash
python3 tools/check-md-links.py --repo-root wavemux --all
python3 tools/check-dir-readmes.py --repo-root wavemux --all
```
