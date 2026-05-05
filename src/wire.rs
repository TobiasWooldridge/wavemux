//! Binary wire format: 12-byte subframe headers + payloads.

use serde::{Deserialize, Serialize};

/// Subframe type discriminant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum SubframeType {
    /// PCM/Opus/IMBE audio samples.
    Audio = 0x01,
    /// A new call/transmission begins. Payload is JSON metadata.
    CallStart = 0x02,
    /// A call/transmission ends. Payload is JSON metadata.
    CallEnd = 0x03,
    /// Substream metadata (sent on subscribe and on change). Payload is JSON.
    StreamInfo = 0x04,
    /// Mid-call metadata update. Payload is JSON; conceptually a diff
    /// against the `CallStart` metadata (UI merges the new fields into
    /// its per-call state). Currently used to surface per-call state
    /// that only becomes observable mid-call:
    ///
    ///   - Encrypted-call detection from LDU2 ESS — `encrypted`,
    ///     `algorithm_id`, `decrypted` fields (WaveCatch#71).
    ///
    /// Clients that don't recognise the frame type should skip it
    /// forward-compatibly — adding new mid-call metadata doesn't
    /// require a major-version bump.
    CallMetadataUpdate = 0x05,
    /// Unit location report (GPS from P25 LRRP or external). Payload is JSON.
    Location = 0x06,
}

impl SubframeType {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0x01 => Some(Self::Audio),
            0x02 => Some(Self::CallStart),
            0x03 => Some(Self::CallEnd),
            0x04 => Some(Self::StreamInfo),
            0x05 => Some(Self::CallMetadataUpdate),
            0x06 => Some(Self::Location),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Audio => "audio",
            Self::CallStart => "call_start",
            Self::CallEnd => "call_end",
            Self::StreamInfo => "stream_info",
            Self::CallMetadataUpdate => "call_metadata_update",
            Self::Location => "location",
        }
    }
}

/// Audio codec tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum Codec {
    /// Mono i16 little-endian, 48 kHz.
    Pcm16Le = 0x00,
    /// Mono i16 little-endian, 8 kHz (raw P25 before resample).
    Pcm16Le8k = 0x01,
    /// Single Opus frame.
    Opus = 0x02,
    /// Raw 88-bit IMBE voice frame.
    ImbeRaw = 0x03,
}

impl Codec {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0x00 => Some(Self::Pcm16Le),
            0x01 => Some(Self::Pcm16Le8k),
            0x02 => Some(Self::Opus),
            0x03 => Some(Self::ImbeRaw),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pcm16Le => "pcm16le",
            Self::Pcm16Le8k => "pcm16le_8k",
            Self::Opus => "opus",
            Self::ImbeRaw => "imbe_raw",
        }
    }

    /// Parse the wire-format string name into a [`Codec`]. Named
    /// `parse_str` rather than `from_str` so it doesn't shadow
    /// `std::str::FromStr::from_str` — clippy's
    /// `should_implement_trait` rule otherwise fires because a
    /// naïve `from_str` method on a type with this signature looks
    /// like a broken `FromStr` impl. The Option return is the right
    /// shape for JSONL parsing where a missing codec short-circuits
    /// the whole frame via `?`, so implementing `FromStr` with
    /// Result<_, Err> would just push `.ok()?` onto every call site.
    pub fn parse_str(s: &str) -> Option<Self> {
        match s {
            "pcm16le" => Some(Self::Pcm16Le),
            "pcm16le_8k" => Some(Self::Pcm16Le8k),
            "opus" => Some(Self::Opus),
            "imbe_raw" => Some(Self::ImbeRaw),
            _ => None,
        }
    }
}

/// Fixed 12-byte header for each subframe within a mux payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SubframeHeader {
    pub substream_id: u16,
    pub subframe_type: SubframeType,
    pub codec: Codec,
    pub source_id: u32,
    pub payload_len: u32,
}

pub const SUBFRAME_HEADER_SIZE: usize = 12;

impl SubframeHeader {
    pub fn encode(&self, buf: &mut [u8; SUBFRAME_HEADER_SIZE]) {
        buf[0..2].copy_from_slice(&self.substream_id.to_be_bytes());
        buf[2] = self.subframe_type as u8;
        buf[3] = self.codec as u8;
        buf[4..8].copy_from_slice(&self.source_id.to_be_bytes());
        buf[8..12].copy_from_slice(&self.payload_len.to_be_bytes());
    }

    pub fn decode(buf: &[u8; SUBFRAME_HEADER_SIZE]) -> Option<Self> {
        let substream_id = u16::from_be_bytes(buf[0..2].try_into().unwrap());
        let subframe_type = SubframeType::from_u8(buf[2])?;
        let codec = Codec::from_u8(buf[3])?;
        let source_id = u32::from_be_bytes(buf[4..8].try_into().unwrap());
        let payload_len = u32::from_be_bytes(buf[8..12].try_into().unwrap());

        Some(Self {
            substream_id,
            subframe_type,
            codec,
            source_id,
            payload_len,
        })
    }
}

/// A decoded mux subframe (header + payload).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Subframe {
    pub substream_id: u16,
    pub subframe_type: SubframeType,
    pub codec: Codec,
    pub source_id: u32,
    pub payload: Vec<u8>,
}

impl Subframe {
    /// Encode this subframe (header + payload) and append to `out`.
    pub fn encode_to(&self, out: &mut Vec<u8>) {
        let header = SubframeHeader {
            substream_id: self.substream_id,
            subframe_type: self.subframe_type,
            codec: self.codec,
            source_id: self.source_id,
            payload_len: self.payload.len() as u32,
        };
        let mut hdr_buf = [0u8; SUBFRAME_HEADER_SIZE];
        header.encode(&mut hdr_buf);
        out.extend_from_slice(&hdr_buf);
        out.extend_from_slice(&self.payload);
    }

    /// Create an audio subframe.
    pub fn audio(substream_id: u16, codec: Codec, source_id: u32, payload: Vec<u8>) -> Self {
        Self {
            substream_id,
            subframe_type: SubframeType::Audio,
            codec,
            source_id,
            payload,
        }
    }

    /// Create a control subframe (CallStart/CallEnd/StreamInfo) with JSON payload.
    pub fn control(
        substream_id: u16,
        subframe_type: SubframeType,
        source_id: u32,
        json: &serde_json::Value,
    ) -> Self {
        Self {
            substream_id,
            subframe_type,
            codec: Codec::Pcm16Le, // unused for control frames
            source_id,
            payload: json.to_string().into_bytes(),
        }
    }
}

/// Encode multiple subframes into a single payload buffer.
pub fn encode_subframes(subframes: &[Subframe]) -> Vec<u8> {
    let total: usize = subframes
        .iter()
        .map(|sf| SUBFRAME_HEADER_SIZE + sf.payload.len())
        .sum();
    let mut buf = Vec::with_capacity(total);
    for sf in subframes {
        sf.encode_to(&mut buf);
    }
    buf
}

/// Decode a payload buffer into subframes.
pub fn decode_subframes(data: &[u8]) -> Vec<Subframe> {
    let mut subframes = Vec::new();
    let mut offset = 0;

    while offset + SUBFRAME_HEADER_SIZE <= data.len() {
        let hdr_bytes: &[u8; SUBFRAME_HEADER_SIZE] = data[offset..offset + SUBFRAME_HEADER_SIZE]
            .try_into()
            .unwrap();
        let Some(header) = SubframeHeader::decode(hdr_bytes) else {
            break;
        };
        offset += SUBFRAME_HEADER_SIZE;

        let payload_len = header.payload_len as usize;
        if offset + payload_len > data.len() {
            break;
        }

        subframes.push(Subframe {
            substream_id: header.substream_id,
            subframe_type: header.subframe_type,
            codec: header.codec,
            source_id: header.source_id,
            payload: data[offset..offset + payload_len].to_vec(),
        });
        offset += payload_len;
    }

    subframes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_roundtrip() {
        let header = SubframeHeader {
            substream_id: 1234,
            subframe_type: SubframeType::Audio,
            codec: Codec::Pcm16Le,
            source_id: 56789,
            payload_len: 640,
        };
        let mut buf = [0u8; SUBFRAME_HEADER_SIZE];
        header.encode(&mut buf);
        let decoded = SubframeHeader::decode(&buf).unwrap();
        assert_eq!(header, decoded);
    }

    #[test]
    fn header_size_is_12() {
        assert_eq!(SUBFRAME_HEADER_SIZE, 12);
    }

    /// Exhaustive on-wire byte pin + compile-time match-gate for
    /// every `SubframeType` variant. Pre-fix this test only iterated
    /// 4 of 6 variants — `CallMetadataUpdate` (0x05, added in
    /// 10bc9a7 for WaveCatch#71) and `Location` (0x06) silently
    /// dropped off the coverage when they landed. A drift-guard
    /// closure forces a future variant addition to fail to build
    /// until both `from_u8` and this iteration are updated.
    ///
    /// The numeric tags are wire-stable: WaveCatch's wavemux
    /// emitter, wavecap's `wcap-ingest::wavemux_jsonl` parser, and
    /// any third-party consumer all read these by raw byte.
    #[test]
    fn subframe_type_wire_byte_assignments() {
        let _exhaustive_check = |st: SubframeType| match st {
            SubframeType::Audio
            | SubframeType::CallStart
            | SubframeType::CallEnd
            | SubframeType::StreamInfo
            | SubframeType::CallMetadataUpdate
            | SubframeType::Location => (),
        };
        let cases: [(SubframeType, u8); 6] = [
            (SubframeType::Audio, 0x01),
            (SubframeType::CallStart, 0x02),
            (SubframeType::CallEnd, 0x03),
            (SubframeType::StreamInfo, 0x04),
            (SubframeType::CallMetadataUpdate, 0x05),
            (SubframeType::Location, 0x06),
        ];
        for (st, byte) in cases {
            assert_eq!(st as u8, byte, "wire byte for {st:?} drifted");
            assert_eq!(
                SubframeType::from_u8(byte),
                Some(st),
                "from_u8({byte:#04x}) lost its mapping to {st:?}",
            );
        }
    }

    /// Mirror for `Codec`: exhaustive byte pin + match-gate. The
    /// existing `all_codecs_roundtrip` covered all four variants but
    /// had no compile-time gate against silently dropping one in a
    /// future refactor.
    #[test]
    fn codec_wire_byte_assignments() {
        let _exhaustive_check = |c: Codec| match c {
            Codec::Pcm16Le | Codec::Pcm16Le8k | Codec::Opus | Codec::ImbeRaw => (),
        };
        let cases: [(Codec, u8); 4] = [
            (Codec::Pcm16Le, 0x00),
            (Codec::Pcm16Le8k, 0x01),
            (Codec::Opus, 0x02),
            (Codec::ImbeRaw, 0x03),
        ];
        for (c, byte) in cases {
            assert_eq!(c as u8, byte, "wire byte for {c:?} drifted");
            assert_eq!(
                Codec::from_u8(byte),
                Some(c),
                "from_u8({byte:#04x}) lost its mapping to {c:?}",
            );
        }
    }

    /// `SubframeType::as_str` is consumed by the JSONL emitter
    /// (`Subframe` → newline-delimited JSON) and round-tripped by
    /// the JSONL parser. Unlike `Codec` (which had `codec_str_roundtrip`)
    /// `SubframeType` had no string-form coverage at all. Pin the
    /// names so a JSONL-side rename ("call_start" → "callstart")
    /// surfaces here instead of as a silent ingest mismatch.
    #[test]
    fn subframe_type_as_str_pins() {
        let cases: [(SubframeType, &str); 6] = [
            (SubframeType::Audio, "audio"),
            (SubframeType::CallStart, "call_start"),
            (SubframeType::CallEnd, "call_end"),
            (SubframeType::StreamInfo, "stream_info"),
            (SubframeType::CallMetadataUpdate, "call_metadata_update"),
            (SubframeType::Location, "location"),
        ];
        for (st, name) in cases {
            assert_eq!(st.as_str(), name);
        }
    }

    #[test]
    fn invalid_type_returns_none() {
        assert!(SubframeType::from_u8(0xFF).is_none());
        assert!(Codec::from_u8(0xFF).is_none());
    }

    #[test]
    fn encode_decode_multiple_subframes() {
        let subframes = vec![
            Subframe::audio(1, Codec::Pcm16Le, 100, vec![0xAA; 640]),
            Subframe::audio(2, Codec::Pcm16Le8k, 200, vec![0xBB; 320]),
            Subframe::control(
                1,
                SubframeType::CallEnd,
                100,
                &serde_json::json!({"duration_ms": 4200}),
            ),
        ];
        let data = encode_subframes(&subframes);
        let decoded = decode_subframes(&data);
        assert_eq!(decoded.len(), 3);
        assert_eq!(decoded[0].substream_id, 1);
        assert_eq!(decoded[0].payload.len(), 640);
        assert_eq!(decoded[1].substream_id, 2);
        assert_eq!(decoded[2].subframe_type, SubframeType::CallEnd);
    }

    #[test]
    fn decode_truncated_stops() {
        let sf = Subframe::audio(1, Codec::Pcm16Le, 0, vec![0; 100]);
        let mut data = encode_subframes(&[sf]);
        data.truncate(SUBFRAME_HEADER_SIZE + 50);
        assert!(decode_subframes(&data).is_empty());
    }

    #[test]
    fn decode_empty() {
        assert!(decode_subframes(&[]).is_empty());
    }

    #[test]
    fn codec_str_roundtrip() {
        for c in [
            Codec::Pcm16Le,
            Codec::Pcm16Le8k,
            Codec::Opus,
            Codec::ImbeRaw,
        ] {
            assert_eq!(Codec::parse_str(c.as_str()), Some(c));
        }
    }
}
