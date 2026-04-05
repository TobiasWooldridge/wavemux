//! Multiplexed audio stream protocol for SDR applications.
//!
//! Packs audio from many substreams (talk groups, channels) into a single
//! connection. Each mux frame contains one or more subframes, each tagged
//! with a substream ID, type, and codec.
//!
//! Silence is free — no subframes are sent for silent substreams.
//!
//! # Wire format
//!
//! Each subframe has a 12-byte header followed by a variable-length payload:
//!
//! ```text
//! Offset  Size  Field
//! 0       2     substream_id    u16 BE  (talkgroup ID or channel index)
//! 2       1     subframe_type   u8      (Audio=1, CallStart=2, CallEnd=3, StreamInfo=4)
//! 3       1     codec           u8      (Pcm16Le=0, Pcm16Le8k=1, Opus=2, ImbeRaw=3)
//! 4       4     source_id       u32 BE  (P25 source unit ID, or 0)
//! 8       4     payload_len     u32 BE  (subframe payload bytes)
//! ```
//!
//! Multiple subframes are concatenated in a single frame.
//!
//! # JSONL format
//!
//! For HTTP consumers, an NDJSON (newline-delimited JSON) format is provided:
//!
//! ```json
//! {"type":"call_start","substream_id":1,"source_id":12345,"data":{...}}
//! {"type":"audio","substream_id":1,"source_id":12345,"codec":"pcm16le","samples_b64":"..."}
//! {"type":"call_end","substream_id":1,"source_id":0,"data":{...}}
//! ```

mod jsonl;
mod wire;

pub use jsonl::*;
pub use wire::*;
