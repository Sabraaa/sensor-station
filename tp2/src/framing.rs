pub const MAX_LINE_BYTES: usize = 1024;

#[derive(Debug)]
pub struct Frame {
    pub bytes: Vec<u8>,
    pub error: Option<&'static str>,
}

/// Bounded line decoder that resynchronizes at LF after oversized input.
#[derive(Default)]
pub struct Decoder {
    bytes: Vec<u8>,
    oversized: bool,
}

impl Decoder {
    /// Return a complete frame at LF and remove an optional preceding CR.
    pub fn push(&mut self, byte: u8) -> Option<Frame> {
        if byte == b'\n' {
            let error = self.oversized.then_some("measurement exceeds 1024 bytes");
            let mut bytes = std::mem::take(&mut self.bytes);
            if bytes.last() == Some(&b'\r') {
                bytes.pop();
            }
            self.oversized = false;
            Some(Frame { bytes, error })
        } else {
            if self.bytes.len() < MAX_LINE_BYTES {
                self.bytes.push(byte);
            } else {
                self.oversized = true;
            }
            None
        }
    }

    /// Reject any unfinished line at end-of-file, disconnect or shutdown.
    pub fn finish(&mut self) -> Option<Frame> {
        if self.bytes.is_empty() && !self.oversized {
            return None;
        }
        self.oversized = false;
        Some(Frame {
            bytes: std::mem::take(&mut self.bytes),
            error: Some("incomplete measurement without newline"),
        })
    }
}
