//! JSONL (newline-delimited JSON) serialization for HTTP consumers.
//!
//! Each line is a self-contained JSON object with a `"type"` field.

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;

use crate::wire::{Codec, Subframe, SubframeType};

/// Serialize a subframe to a JSONL line (without trailing newline).
pub fn subframe_to_jsonl(sf: &Subframe) -> String {
    match sf.subframe_type {
        SubframeType::Audio => serde_json::json!({
            "type": "audio",
            "substream_id": sf.substream_id,
            "source_id": sf.source_id,
            "codec": sf.codec.as_str(),
            "samples_b64": BASE64.encode(&sf.payload),
        })
        .to_string(),
        _ => {
            let data: serde_json::Value =
                serde_json::from_slice(&sf.payload).unwrap_or(serde_json::json!({}));
            serde_json::json!({
                "type": sf.subframe_type.as_str(),
                "substream_id": sf.substream_id,
                "source_id": sf.source_id,
                "data": data,
            })
            .to_string()
        }
    }
}

/// Parse a JSONL line back into a subframe.
pub fn jsonl_to_subframe(line: &str) -> Option<Subframe> {
    let v: serde_json::Value = serde_json::from_str(line).ok()?;
    let type_str = v.get("type")?.as_str()?;
    let substream_id = v.get("substream_id")?.as_u64()? as u16;
    let source_id = v.get("source_id")?.as_u64().unwrap_or(0) as u32;

    match type_str {
        "audio" => {
            let codec_str = v.get("codec")?.as_str()?;
            let codec = Codec::from_str(codec_str)?;
            let b64 = v.get("samples_b64")?.as_str()?;
            let payload = BASE64.decode(b64).ok()?;
            Some(Subframe {
                substream_id,
                subframe_type: SubframeType::Audio,
                codec,
                source_id,
                payload,
            })
        }
        "call_start" | "call_end" | "stream_info" => {
            let subframe_type = match type_str {
                "call_start" => SubframeType::CallStart,
                "call_end" => SubframeType::CallEnd,
                "stream_info" => SubframeType::StreamInfo,
                _ => return None,
            };
            let data = v.get("data").cloned().unwrap_or(serde_json::json!({}));
            Some(Subframe {
                substream_id,
                subframe_type,
                codec: Codec::Pcm16Le,
                source_id,
                payload: data.to_string().into_bytes(),
            })
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audio_roundtrip() {
        let sf = Subframe::audio(42, Codec::Pcm16Le, 12345, vec![1, 2, 3, 4]);
        let line = subframe_to_jsonl(&sf);
        let parsed = jsonl_to_subframe(&line).unwrap();
        assert_eq!(parsed.substream_id, 42);
        assert_eq!(parsed.subframe_type, SubframeType::Audio);
        assert_eq!(parsed.codec, Codec::Pcm16Le);
        assert_eq!(parsed.source_id, 12345);
        assert_eq!(parsed.payload, vec![1, 2, 3, 4]);
    }

    #[test]
    fn call_start_roundtrip() {
        let metadata = serde_json::json!({
            "talkgroup_name": "Dispatch",
            "frequency_hz": 851062500.0,
        });
        let sf = Subframe::control(1, SubframeType::CallStart, 100, &metadata);
        let line = subframe_to_jsonl(&sf);
        let parsed = jsonl_to_subframe(&line).unwrap();
        assert_eq!(parsed.substream_id, 1);
        assert_eq!(parsed.subframe_type, SubframeType::CallStart);
        assert_eq!(parsed.source_id, 100);
        // Parse embedded JSON
        let data: serde_json::Value = serde_json::from_slice(&parsed.payload).unwrap();
        assert_eq!(data["talkgroup_name"], "Dispatch");
    }

    #[test]
    fn call_end_roundtrip() {
        let sf = Subframe::control(
            5,
            SubframeType::CallEnd,
            0,
            &serde_json::json!({"duration_ms": 4200}),
        );
        let line = subframe_to_jsonl(&sf);
        let parsed = jsonl_to_subframe(&line).unwrap();
        assert_eq!(parsed.subframe_type, SubframeType::CallEnd);
        let data: serde_json::Value = serde_json::from_slice(&parsed.payload).unwrap();
        assert_eq!(data["duration_ms"], 4200);
    }

    #[test]
    fn invalid_json_returns_none() {
        assert!(jsonl_to_subframe("not json").is_none());
    }

    #[test]
    fn unknown_type_returns_none() {
        assert!(jsonl_to_subframe(r#"{"type":"unknown","substream_id":1}"#).is_none());
    }

    #[test]
    fn jsonl_is_single_line() {
        let sf = Subframe::audio(1, Codec::Pcm16Le, 0, vec![0; 100]);
        let line = subframe_to_jsonl(&sf);
        assert!(!line.contains('\n'));
    }
}
