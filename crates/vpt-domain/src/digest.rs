#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Sha256Digest(pub [u8; 32]);

impl Sha256Digest {
    pub fn hex(&self) -> String {
        self.0.iter().map(|byte| format!("{byte:02x}")).collect()
    }

    pub fn hash12(&self) -> String {
        self.hex()[..12].to_owned()
    }

    pub fn hash8(&self) -> String {
        self.hex()[..8].to_owned()
    }

    pub fn from_hex(text: &str) -> Option<Self> {
        if text.len() != 64 || !text.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return None;
        }
        let mut bytes = [0u8; 32];
        for (index, chunk) in text.as_bytes().chunks(2).enumerate() {
            bytes[index] = u8::from_str_radix(std::str::from_utf8(chunk).ok()?, 16).ok()?;
        }
        Some(Self(bytes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_round_trips_with_exact_prefixes() {
        let mut bytes = [0u8; 32];
        bytes[..6].copy_from_slice(&[0x4f, 0x3a, 0xb1, 0x9c, 0x02, 0xde]);
        let digest = Sha256Digest(bytes);
        let hex = digest.hex();
        assert_eq!(hex.len(), 64);
        assert!(hex.starts_with("4f3ab19c02de"));
        assert_eq!(digest.hash12(), "4f3ab19c02de");
        assert_eq!(digest.hash8(), "4f3ab19c");
        assert_eq!(Sha256Digest::from_hex(&hex), Some(digest));
    }

    #[test]
    fn full_length_malformed_and_non_ascii_hex_are_refused() {
        for text in [
            "zz".to_owned(),
            "ab".repeat(31),
            format!("{}zz", "ab".repeat(31)),
            format!("{}é", "ab".repeat(31)),
        ] {
            assert_eq!(Sha256Digest::from_hex(&text), None);
        }
    }
}
