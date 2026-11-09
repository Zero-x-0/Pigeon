use std::fs::File;
use std::io::Read;
use std::path::Path;
use usb_x_core::errors::{Result, UsbXError};

#[derive(Debug, Clone, Default)]
pub struct ImageChecksums {
    pub sha256: String,
    pub md5: String,
    pub crc32: String,
}

pub fn calculate_checksums<F>(path_str: &str, mut progress: F) -> Result<ImageChecksums>
where
    F: FnMut(u64, u64),
{
    let path = Path::new(path_str);
    let mut file = File::open(path).map_err(|e| UsbXError::Io {
        operation: "open_image_for_hash".to_string(),
        message: e.to_string(),
    })?;

    let total_size = file.metadata().map_err(|e| UsbXError::Io {
        operation: "metadata".to_string(),
        message: e.to_string(),
    })?.len();

    let mut sha256 = Sha256::new();
    let mut crc32 = Crc32::new();
    let mut md5 = Md5::new();

    let mut buffer = vec![0u8; 4 * 1024 * 1024]; // 4 MiB buffer
    let mut processed: u64 = 0;

    loop {
        let bytes_read = file.read(&mut buffer).map_err(|e| UsbXError::Io {
            operation: "read_hash_chunk".to_string(),
            message: e.to_string(),
        })?;

        if bytes_read == 0 {
            break;
        }

        let chunk = &buffer[..bytes_read];
        sha256.update(chunk);
        crc32.update(chunk);
        md5.update(chunk);

        processed += bytes_read as u64;
        progress(processed, total_size);
    }

    Ok(ImageChecksums {
        sha256: sha256.finalize_hex(),
        md5: md5.finalize_hex(),
        crc32: format!("{:08X}", crc32.finalize()),
    })
}

pub fn verify_hash(actual: &str, expected: &str) -> bool {
    actual.trim().eq_ignore_ascii_case(expected.trim())
}

// --- High-Speed IEEE 802.3 CRC32 with 256-Entry Table ---
const CRC_TABLE: [u32; 256] = {
    let mut table = [0u32; 256];
    let mut i = 0;
    while i < 256 {
        let mut c = i as u32;
        let mut j = 0;
        while j < 8 {
            if (c & 1) != 0 {
                c = 0xEDB8_8320 ^ (c >> 1);
            } else {
                c >>= 1;
            }
            j += 1;
        }
        table[i] = c;
        i += 1;
    }
    table
};

pub struct Crc32 {
    state: u32,
}

impl Crc32 {
    pub fn new() -> Self {
        Self { state: 0xFFFF_FFFF }
    }

    #[inline]
    pub fn update(&mut self, data: &[u8]) {
        let mut crc = self.state;
        for &byte in data {
            crc = CRC_TABLE[((crc ^ (byte as u32)) & 0xFF) as usize] ^ (crc >> 8);
        }
        self.state = crc;
    }

    pub fn finalize(&self) -> u32 {
        !self.state
    }
}

// --- Fast Standard RFC 1321 MD5 ---
pub struct Md5 {
    state: [u32; 4],
    count: u64,
    buffer: [u8; 64],
    buffer_len: usize,
}

impl Md5 {
    pub fn new() -> Self {
        Self {
            state: [0x67452301, 0xefcdab89, 0x98badcfe, 0x10325476],
            count: 0,
            buffer: [0u8; 64],
            buffer_len: 0,
        }
    }

    pub fn update(&mut self, mut input: &[u8]) {
        self.count += input.len() as u64;

        if self.buffer_len > 0 {
            let to_copy = (64 - self.buffer_len).min(input.len());
            self.buffer[self.buffer_len..self.buffer_len + to_copy].copy_from_slice(&input[..to_copy]);
            self.buffer_len += to_copy;
            input = &input[to_copy..];

            if self.buffer_len == 64 {
                let buf = self.buffer;
                self.transform(&buf);
                self.buffer_len = 0;
            }
        }

        while input.len() >= 64 {
            let block: [u8; 64] = input[..64].try_into().unwrap();
            self.transform(&block);
            input = &input[64..];
        }

        if !input.is_empty() {
            self.buffer[..input.len()].copy_from_slice(input);
            self.buffer_len = input.len();
        }
    }

    fn transform(&mut self, block: &[u8; 64]) {
        let mut x = [0u32; 16];
        for i in 0..16 {
            x[i] = u32::from_le_bytes(block[i * 4..i * 4 + 4].try_into().unwrap());
        }

        let mut a = self.state[0];
        let mut b = self.state[1];
        let mut c = self.state[2];
        let mut d = self.state[3];

        macro_rules! ff {
            ($a:ident, $b:ident, $c:ident, $d:ident, $k:expr, $s:expr, $i:expr) => {
                $a = $a.wrapping_add(($b & $c) | ((!$b) & $d)).wrapping_add(x[$k]).wrapping_add($i);
                $a = $a.rotate_left($s).wrapping_add($b);
            };
        }
        macro_rules! gg {
            ($a:ident, $b:ident, $c:ident, $d:ident, $k:expr, $s:expr, $i:expr) => {
                $a = $a.wrapping_add(($b & $d) | ($c & (!$d))).wrapping_add(x[$k]).wrapping_add($i);
                $a = $a.rotate_left($s).wrapping_add($b);
            };
        }
        macro_rules! hh {
            ($a:ident, $b:ident, $c:ident, $d:ident, $k:expr, $s:expr, $i:expr) => {
                $a = $a.wrapping_add($b ^ $c ^ $d).wrapping_add(x[$k]).wrapping_add($i);
                $a = $a.rotate_left($s).wrapping_add($b);
            };
        }
        macro_rules! ii {
            ($a:ident, $b:ident, $c:ident, $d:ident, $k:expr, $s:expr, $i:expr) => {
                $a = $a.wrapping_add($c ^ ($b | (!$d))).wrapping_add(x[$k]).wrapping_add($i);
                $a = $a.rotate_left($s).wrapping_add($b);
            };
        }

        // Round 1
        ff!(a, b, c, d, 0, 7, 0xd76aa478); ff!(d, a, b, c, 1, 12, 0xe8c7b756);
        ff!(c, d, a, b, 2, 17, 0x242070db); ff!(b, c, d, a, 3, 22, 0xc1bdceee);
        ff!(a, b, c, d, 4, 7, 0xf57c0faf); ff!(d, a, b, c, 5, 12, 0x4787c62a);
        ff!(c, d, a, b, 6, 17, 0xa8304613); ff!(b, c, d, a, 7, 22, 0xfd469501);
        ff!(a, b, c, d, 8, 7, 0x698098d8); ff!(d, a, b, c, 9, 12, 0x8b44f7af);
        ff!(c, d, a, b, 10, 17, 0xffff5bb1); ff!(b, c, d, a, 11, 22, 0x895cd7be);
        ff!(a, b, c, d, 12, 7, 0x6b901122); ff!(d, a, b, c, 13, 12, 0xfd987193);
        ff!(c, d, a, b, 14, 17, 0xa679438e); ff!(b, c, d, a, 15, 22, 0x49b40821);

        // Round 2
        gg!(a, b, c, d, 1, 5, 0xf61e2562); gg!(d, a, b, c, 6, 9, 0xc040b340);
        gg!(c, d, a, b, 11, 14, 0x265e5a51); gg!(b, c, d, a, 0, 20, 0xe9b6c7aa);
        gg!(a, b, c, d, 5, 5, 0xd62f105d); gg!(d, a, b, c, 10, 9, 0x02441453);
        gg!(c, d, a, b, 15, 14, 0xd8a1e681); gg!(b, c, d, a, 4, 20, 0xe7d3fbc8);
        gg!(a, b, c, d, 9, 5, 0x21e1cde6); gg!(d, a, b, c, 14, 9, 0xc33707d6);
        gg!(c, d, a, b, 3, 14, 0xf4d50d87); gg!(b, c, d, a, 8, 20, 0x455a14ed);
        gg!(a, b, c, d, 13, 5, 0xa9e3e905); gg!(d, a, b, c, 2, 9, 0xfcefa3f8);
        gg!(c, d, a, b, 7, 14, 0x676f02d9); gg!(b, c, d, a, 12, 20, 0x8d2a4c8a);

        // Round 3
        hh!(a, b, c, d, 5, 4, 0xfffa3942); hh!(d, a, b, c, 8, 11, 0x8771f681);
        hh!(c, d, a, b, 11, 16, 0x6d9d6122); hh!(b, c, d, a, 14, 23, 0xfde5380c);
        hh!(a, b, c, d, 1, 4, 0xa4beea44); hh!(d, a, b, c, 4, 11, 0x4bdecfa9);
        hh!(c, d, a, b, 7, 16, 0xf6bb4b60); hh!(b, c, d, a, 10, 23, 0xbebfbc70);
        hh!(a, b, c, d, 13, 4, 0x289b7ec6); hh!(d, a, b, c, 0, 11, 0xeaa127fa);
        hh!(c, d, a, b, 3, 16, 0xd4ef3085); hh!(b, c, d, a, 6, 23, 0x04881d05);
        hh!(a, b, c, d, 9, 4, 0xd9d4d039); hh!(d, a, b, c, 12, 11, 0xe6db99e5);
        hh!(c, d, a, b, 15, 16, 0x1fa27cf8); hh!(b, c, d, a, 2, 23, 0xc4ac5665);

        // Round 4
        ii!(a, b, c, d, 0, 6, 0xf4292244); ii!(d, a, b, c, 7, 10, 0x432aff97);
        ii!(c, d, a, b, 14, 15, 0xab9423a7); ii!(b, c, d, a, 5, 21, 0xfc93a039);
        ii!(a, b, c, d, 12, 6, 0x655b59c3); ii!(d, a, b, c, 3, 10, 0x8f0ccc92);
        ii!(c, d, a, b, 10, 15, 0xffeff47d); ii!(b, c, d, a, 1, 21, 0x85845dd1);
        ii!(a, b, c, d, 8, 6, 0x6fa87e4f); ii!(d, a, b, c, 15, 10, 0xfe2ce6e0);
        ii!(c, d, a, b, 6, 15, 0xa3014314); ii!(b, c, d, a, 13, 21, 0x4e0811a1);
        ii!(a, b, c, d, 4, 6, 0xf7537e82); ii!(d, a, b, c, 11, 10, 0xbd3af235);
        ii!(c, d, a, b, 2, 15, 0x2ad7d2bb); ii!(b, c, d, a, 9, 21, 0xeb86d391);

        self.state[0] = self.state[0].wrapping_add(a);
        self.state[1] = self.state[1].wrapping_add(b);
        self.state[2] = self.state[2].wrapping_add(c);
        self.state[3] = self.state[3].wrapping_add(d);
    }

    pub fn finalize_hex(&mut self) -> String {
        let mut pad = [0u8; 64];
        pad[0] = 0x80;

        let bits = self.count * 8;
        let bits_le = bits.to_le_bytes();

        let left = (self.count & 0x3F) as usize;
        let padlen = if left < 56 { 56 - left } else { 120 - left };
        self.update(&pad[..padlen]);
        self.update(&bits_le);

        let mut out = [0u8; 16];
        for i in 0..4 {
            out[i * 4..i * 4 + 4].copy_from_slice(&self.state[i].to_le_bytes());
        }

        let mut s = String::with_capacity(32);
        for b in out {
            s.push_str(&format!("{:02x}", b));
        }
        s
    }
}

// --- Zero-dependency Standard FIPS 180-4 SHA-256 ---
pub struct Sha256 {
    state: [u32; 8],
    count: u64,
    buffer: [u8; 64],
    buffer_len: usize,
}

impl Sha256 {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
        0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
        0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
        0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
        0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
        0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
        0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
    ];

    pub fn new() -> Self {
        Self {
            state: [
                0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
                0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
            ],
            count: 0,
            buffer: [0u8; 64],
            buffer_len: 0,
        }
    }

    pub fn update(&mut self, mut input: &[u8]) {
        self.count += input.len() as u64;

        if self.buffer_len > 0 {
            let to_copy = (64 - self.buffer_len).min(input.len());
            self.buffer[self.buffer_len..self.buffer_len + to_copy].copy_from_slice(&input[..to_copy]);
            self.buffer_len += to_copy;
            input = &input[to_copy..];

            if self.buffer_len == 64 {
                let buf = self.buffer;
                self.transform(&buf);
                self.buffer_len = 0;
            }
        }

        while input.len() >= 64 {
            let block: [u8; 64] = input[..64].try_into().unwrap();
            self.transform(&block);
            input = &input[64..];
        }

        if !input.is_empty() {
            self.buffer[..input.len()].copy_from_slice(input);
            self.buffer_len = input.len();
        }
    }

    fn transform(&mut self, block: &[u8; 64]) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes(block[i * 4..i * 4 + 4].try_into().unwrap());
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16].wrapping_add(s0).wrapping_add(w[i - 7]).wrapping_add(s1);
        }

        let mut a = self.state[0];
        let mut b = self.state[1];
        let mut c = self.state[2];
        let mut d = self.state[3];
        let mut e = self.state[4];
        let mut f = self.state[5];
        let mut g = self.state[6];
        let mut h = self.state[7];

        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = h.wrapping_add(s1).wrapping_add(ch).wrapping_add(Self::K[i]).wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);

            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        self.state[0] = self.state[0].wrapping_add(a);
        self.state[1] = self.state[1].wrapping_add(b);
        self.state[2] = self.state[2].wrapping_add(c);
        self.state[3] = self.state[3].wrapping_add(d);
        self.state[4] = self.state[4].wrapping_add(e);
        self.state[5] = self.state[5].wrapping_add(f);
        self.state[6] = self.state[6].wrapping_add(g);
        self.state[7] = self.state[7].wrapping_add(h);
    }

    pub fn finalize_hex(mut self) -> String {
        let bit_len = self.count * 8;
        self.buffer[self.buffer_len] = 0x80;
        self.buffer_len += 1;

        if self.buffer_len > 56 {
            for i in self.buffer_len..64 {
                self.buffer[i] = 0;
            }
            let buf = self.buffer;
            self.transform(&buf);
            self.buffer_len = 0;
        }

        for i in self.buffer_len..56 {
            self.buffer[i] = 0;
        }

        self.buffer[56..64].copy_from_slice(&bit_len.to_be_bytes());
        let buf = self.buffer;
        self.transform(&buf);

        let mut hex = String::with_capacity(64);
        for word in self.state {
            use std::fmt::Write;
            let _ = write!(&mut hex, "{:08x}", word);
        }
        hex
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sha256_empty() {
        let hasher = Sha256::new();
        let hash = hasher.finalize_hex();
        assert_eq!(hash, "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
    }

    #[test]
    fn test_sha256_known_string() {
        let mut hasher = Sha256::new();
        hasher.update(b"hello");
        let hash = hasher.finalize_hex();
        assert_eq!(hash, "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824");
    }

    #[test]
    fn test_crc32_check() {
        let mut crc = Crc32::new();
        crc.update(b"123456789");
        let val = crc.finalize();
        assert_eq!(format!("{:08X}", val), "CBF43926");
    }

    #[test]
    fn test_md5_known_string() {
        let mut md5 = Md5::new();
        md5.update(b"The quick brown fox jumps over the lazy dog");
        assert_eq!(md5.finalize_hex(), "9e107d9d372bb6826bd81d3542a419d6");
    }
}
