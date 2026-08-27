//! Digests, and which artifact takes which.
//!
//! PB §11, *Hash policy*: "Git object ids (`<oid>`, in the repo's object format)
//! for everything that is a git object: intent blob, frozen files, trees,
//! commits. SHA-256 (`sha256:<hex>`) only for non-git artifacts: release
//! artifact list (`dist_hash`), gate report, freeze digest, envelope digest,
//! B's transcript."
//!
//! The two are not interchangeable and the prefix is what keeps them apart in
//! every serialized form.

use sha2::{Digest, Sha256};

/// Lowercase hex SHA-256, unprefixed. Used where the surrounding format
/// supplies its own framing — `sha256sum` lines in the release artifact list
/// (CI §5.5), for instance.
pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(64);
    for byte in digest {
        out.push(hex_lower(byte >> 4));
        out.push(hex_lower(byte & 0xF));
    }
    out
}

/// `sha256:<64 lowercase hex>` — the spelling every spine artifact uses for a
/// non-git digest.
pub fn sha256_prefixed(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(71);
    out.push_str("sha256:");
    out.push_str(&sha256_hex(bytes));
    out
}

/// Parse a `sha256:<hex>` value back to its 32 bytes.
///
/// Strict about case and length because the digest appears inside signed lines:
/// two spellings of one digest would be two byte-different signatures over one
/// fact.
pub fn parse_sha256_prefixed(s: &str) -> Option<[u8; 32]> {
    let hex = s.strip_prefix("sha256:")?;
    parse_sha256_hex(hex)
}

pub fn parse_sha256_hex(hex: &str) -> Option<[u8; 32]> {
    if hex.len() != 64 {
        return None;
    }
    let bytes = hex.as_bytes();
    let mut out = [0u8; 32];
    for (i, slot) in out.iter_mut().enumerate() {
        let hi = from_hex_lower(bytes[i * 2])?;
        let lo = from_hex_lower(bytes[i * 2 + 1])?;
        *slot = (hi << 4) | lo;
    }
    Some(out)
}

/// The git blob id of `content`, in the repository's object format.
///
/// PB §6.7 records `blob` as "the git blob id of what spine wrote
/// (`git hash-object --path`, so `.gitattributes` and CRLF churn are not
/// drift)". The `--path` half — running the content through the repository's
/// filters first — is the caller's job; this computes the id of the bytes it is
/// given, which is what git stores.
pub fn git_blob_id(content: &[u8], format: ObjectFormat) -> String {
    let header = format!("blob {}\0", content.len());
    match format {
        ObjectFormat::Sha1 => {
            let mut hasher = Sha1::new();
            hasher.update(header.as_bytes());
            hasher.update(content);
            hasher.hex()
        }
        ObjectFormat::Sha256 => {
            let mut hasher = Sha256::new();
            hasher.update(header.as_bytes());
            hasher.update(content);
            let digest = hasher.finalize();
            let mut out = String::with_capacity(64);
            for byte in digest {
                out.push(hex_lower(byte >> 4));
                out.push(hex_lower(byte & 0xF));
            }
            out
        }
    }
}

/// The repository's object format, read from the manifest's `object_format`
/// member (PB §6.7 — one of its twelve frozen fields).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectFormat {
    Sha1,
    Sha256,
}

impl ObjectFormat {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "sha1" => Some(ObjectFormat::Sha1),
            "sha256" => Some(ObjectFormat::Sha256),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            ObjectFormat::Sha1 => "sha1",
            ObjectFormat::Sha256 => "sha256",
        }
    }

    /// Hex digits in an object id of this format. Used to reject an oid of the
    /// wrong width before it reaches a comparison that would silently fail.
    pub fn hex_len(self) -> usize {
        match self {
            ObjectFormat::Sha1 => 40,
            ObjectFormat::Sha256 => 64,
        }
    }
}

fn hex_lower(nibble: u8) -> char {
    match nibble {
        0..=9 => (b'0' + nibble) as char,
        _ => (b'a' + (nibble - 10)) as char,
    }
}

fn from_hex_lower(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        _ => None,
    }
}

/// SHA-1, for `object_format: sha1` repositories only.
///
/// Present because git's default object format is still SHA-1 and a blob id is
/// how the manifest records what spine wrote — it is never used as a security
/// primitive here. Every *security* digest in the corpus is SHA-256 (PB §11,
/// *Hash policy*), and the one place a blob id carries weight, G16's
/// tamper check, is bounded by the seal's SHA-256 envelope digest above it.
mod sha1 {
    pub struct Sha1 {
        state: [u32; 5],
        buffer: [u8; 64],
        buffered: usize,
        length_bits: u64,
    }

    impl Sha1 {
        pub fn new() -> Self {
            Sha1 {
                state: [0x67452301, 0xEFCDAB89, 0x98BADCFE, 0x10325476, 0xC3D2E1F0],
                buffer: [0u8; 64],
                buffered: 0,
                length_bits: 0,
            }
        }

        pub fn update(&mut self, mut data: &[u8]) {
            self.length_bits = self.length_bits.wrapping_add((data.len() as u64) * 8);
            if self.buffered > 0 {
                let take = core::cmp::min(64 - self.buffered, data.len());
                self.buffer[self.buffered..self.buffered + take].copy_from_slice(&data[..take]);
                self.buffered += take;
                data = &data[take..];
                if self.buffered == 64 {
                    let block = self.buffer;
                    self.compress(&block);
                    self.buffered = 0;
                }
            }
            while data.len() >= 64 {
                let (block, rest) = data.split_at(64);
                let mut fixed = [0u8; 64];
                fixed.copy_from_slice(block);
                self.compress(&fixed);
                data = rest;
            }
            if !data.is_empty() {
                self.buffer[..data.len()].copy_from_slice(data);
                self.buffered = data.len();
            }
        }

        pub fn hex(mut self) -> String {
            let length_bits = self.length_bits;
            self.update_raw(&[0x80]);
            while self.buffered != 56 {
                self.update_raw(&[0x00]);
            }
            self.update_raw(&length_bits.to_be_bytes());

            let mut out = String::with_capacity(40);
            for word in self.state {
                for byte in word.to_be_bytes() {
                    out.push(super::hex_lower(byte >> 4));
                    out.push(super::hex_lower(byte & 0xF));
                }
            }
            out
        }

        /// `update` without touching the length counter — padding must not be
        /// counted in the length it encodes.
        fn update_raw(&mut self, data: &[u8]) {
            for &byte in data {
                self.buffer[self.buffered] = byte;
                self.buffered += 1;
                if self.buffered == 64 {
                    let block = self.buffer;
                    self.compress(&block);
                    self.buffered = 0;
                }
            }
        }

        fn compress(&mut self, block: &[u8; 64]) {
            let mut w = [0u32; 80];
            for (i, slot) in w.iter_mut().take(16).enumerate() {
                *slot = u32::from_be_bytes([
                    block[i * 4],
                    block[i * 4 + 1],
                    block[i * 4 + 2],
                    block[i * 4 + 3],
                ]);
            }
            for i in 16..80 {
                w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
            }

            let [mut a, mut b, mut c, mut d, mut e] = self.state;
            for (i, &wi) in w.iter().enumerate() {
                let (f, k) = match i {
                    0..=19 => ((b & c) | ((!b) & d), 0x5A827999u32),
                    20..=39 => (b ^ c ^ d, 0x6ED9EBA1),
                    40..=59 => ((b & c) | (b & d) | (c & d), 0x8F1BBCDC),
                    _ => (b ^ c ^ d, 0xCA62C1D6),
                };
                let temp = a
                    .rotate_left(5)
                    .wrapping_add(f)
                    .wrapping_add(e)
                    .wrapping_add(k)
                    .wrapping_add(wi);
                e = d;
                d = c;
                c = b.rotate_left(30);
                b = a;
                a = temp;
            }

            self.state[0] = self.state[0].wrapping_add(a);
            self.state[1] = self.state[1].wrapping_add(b);
            self.state[2] = self.state[2].wrapping_add(c);
            self.state[3] = self.state[3].wrapping_add(d);
            self.state[4] = self.state[4].wrapping_add(e);
        }
    }
}

use sha1::Sha1;

/// Raw 32-byte SHA-256, for callers that need the digest bytes rather than
/// their hex — an SSH key fingerprint is base64 over these, not over the hex.
pub fn sha256_raw(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_of_empty_is_the_nist_value() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn sha1_matches_the_nist_vectors() {
        // The two FIPS 180-1 examples, plus the empty string.
        struct Case(&'static [u8], &'static str);
        for Case(input, expected) in [
            Case(b"", "da39a3ee5e6b4b0d3255bfef95601890afd80709"),
            Case(b"abc", "a9993e364706816aba3e25717850c26c9cd0d89d"),
            Case(
                b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq",
                "84983e441c3bd26ebaae4aa1f95129e5e54670f1",
            ),
        ] {
            let mut hasher = super::Sha1::new();
            hasher.update(input);
            assert_eq!(hasher.hex(), expected, "SHA-1 of {input:?}");
        }
    }

    /// The blob id must be git's own, or the manifest records a value `git
    /// hash-object` disagrees with and every `spine init` re-run sees drift.
    /// Checked against `git hash-object` in the crate's integration tests; here
    /// the well-known constants suffice.
    #[test]
    fn git_blob_id_matches_git_hash_object() {
        // `printf '' | git hash-object --stdin`
        assert_eq!(
            git_blob_id(b"", ObjectFormat::Sha1),
            "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391"
        );
        // `printf 'hello\n' | git hash-object --stdin`
        assert_eq!(
            git_blob_id(b"hello\n", ObjectFormat::Sha1),
            "ce013625030ba8dba906f756967f9e9ca394464a"
        );
        // The same two in a `git init --object-format=sha256` repository,
        // reproduced against git 2.50.1 rather than derived on paper.
        assert_eq!(
            git_blob_id(b"", ObjectFormat::Sha256),
            "473a0f4c3be8a93681a267e3b1e9a7dcda1185436fe141f7749120a303721813"
        );
        assert_eq!(
            git_blob_id(b"hello\n", ObjectFormat::Sha256),
            "2cf8d83d9ee29543b34a87727421fdecb7e3f3a183d337639025de576db9ebb4"
        );
    }

    #[test]
    fn sha1_spans_the_block_boundary() {
        // 64 bytes exactly, then 63, then 65 — the three cases the buffering
        // in `update` gets wrong if it is wrong at all.
        for len in [55usize, 56, 63, 64, 65, 119, 120, 128] {
            let input = vec![b'a'; len];
            let mut hasher = super::Sha1::new();
            hasher.update(&input);
            let all_at_once = hasher.hex();

            let mut hasher = super::Sha1::new();
            for chunk in input.chunks(7) {
                hasher.update(chunk);
            }
            assert_eq!(hasher.hex(), all_at_once, "chunked SHA-1 differs at {len}");
        }
    }

    #[test]
    fn digest_prefixes_round_trip() {
        let d = sha256_prefixed(b"abc");
        assert!(d.starts_with("sha256:"));
        assert!(parse_sha256_prefixed(&d).is_some());
        // Uppercase hex is not accepted: a signed line carries one spelling.
        assert!(parse_sha256_prefixed(&d.to_uppercase()).is_none());
        assert!(parse_sha256_prefixed("sha256:abc").is_none());
        assert!(parse_sha256_prefixed(&sha256_hex(b"abc")).is_none());
    }
}
