# wavemux Protocol Reference

This document is the reviewable protocol map for `wavemux`. The executable
source of truth remains [`../src/wire.rs`](../src/wire.rs) for the binary
format and [`../src/jsonl.rs`](../src/jsonl.rs) for the newline-delimited JSON
adapter.

## Transport Model

`wavemux` carries many radio substreams over one producer-to-consumer stream.
Each active substream is identified by `substream_id`; silent substreams emit
no frames. The same logical subframe can be carried in either binary form or
JSONL form:

- Binary is compact and consists of concatenated subframes.
- JSONL is line-oriented and easier for HTTP consumers to parse and inspect.

## Binary Subframes

A binary payload is a concatenation of zero or more subframes. Every subframe
starts with a fixed 12-byte big-endian header followed by `payload_len` bytes.

| Offset | Size | Field | Notes |
| --- | --- | --- | --- |
| 0 | 2 | `substream_id` | Talkgroup, channel index, or producer-defined substream key. |
| 2 | 1 | `subframe_type` | Numeric value from the frame type table. |
| 3 | 1 | `codec` | Audio codec tag; ignored for control frames. |
| 4 | 4 | `source_id` | P25 source unit ID or `0` when not applicable. |
| 8 | 4 | `payload_len` | Payload byte count after this header. |

Decoders stop at the first unknown frame type, unknown codec, or truncated
payload and return the subframes decoded so far. Producers should avoid
emitting partial subframes because consumers treat truncation as end-of-frame
data, not as a recoverable inline error.

## Frame Types

| Value | JSONL type | Payload |
| --- | --- | --- |
| `0x01` | `audio` | Raw audio bytes encoded according to `codec`. |
| `0x02` | `call_start` | JSON object with start metadata for a transmission or trunked call. |
| `0x03` | `call_end` | JSON object with end metadata for a transmission or trunked call. |
| `0x04` | `stream_info` | JSON object describing a substream. |
| `0x05` | `call_metadata_update` | JSON object diff to merge into current call metadata. |
| `0x06` | `location` | JSON object containing unit location metadata. |

`call_metadata_update` is for state that becomes visible after `call_start`,
such as encrypted-call detection from P25 LDU2 ESS. Consumers should merge
present fields into the active call state and leave absent fields unchanged.

## Codecs

| Value | JSONL codec | Meaning |
| --- | --- | --- |
| `0x00` | `pcm16le` | Mono signed 16-bit little-endian PCM at 48 kHz. |
| `0x01` | `pcm16le_8k` | Mono signed 16-bit little-endian PCM at 8 kHz. |
| `0x02` | `opus` | One Opus frame. |
| `0x03` | `imbe_raw` | One raw 88-bit IMBE voice frame. |

Control frames currently encode `codec` as `pcm16le`; consumers should ignore
the field unless `subframe_type` is `audio`.

## JSONL Shape

Each JSONL line is a complete JSON object with a `type` field. Audio frames
carry base64 payload bytes:

```json
{"type":"audio","substream_id":1,"source_id":12345,"codec":"pcm16le","samples_b64":"..."}
```

Control frames carry metadata in `data`:

```json
{"type":"call_start","substream_id":1,"source_id":12345,"data":{"talkgroup_name":"Dispatch"}}
```

The parser returns no frame for invalid JSON, missing required fields, unknown
types, unknown codecs, or invalid base64. Empty control `data` defaults to an
empty object.

## Compatibility Notes

- Additive metadata fields inside JSON payloads should be treated as
  forward-compatible.
- New frame types require consumers to decide whether they can skip or must
  reject the value.
- New codecs require explicit consumer support because audio payload decoding
  depends on the codec tag.
- Producers should keep `source_id = 0` for streams where no source unit ID is
  available.
