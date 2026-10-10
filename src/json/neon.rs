//! NEON byte-scan helpers for Apple Silicon / AArch64 JSON lexer hot paths.
//!
//! Mirrors [`super::simd`] (AVX2 on `x86_64`). On non-`aarch64` targets every
//! entry point reports unavailable and falls back to scalar.

use super::simd::{find_non_whitespace_scalar, find_string_special_scalar};

/// True when NEON vectorized byte scan is available (AArch64 baseline).
#[inline]
pub fn is_available() -> bool {
    cfg!(target_arch = "aarch64")
}

/// Index of the first JSON string "special" byte in `haystack`, or `None` if
/// every byte is safe unescaped content.
///
/// A special byte is `"`, `\`, or a control byte (`< 0x20`).
#[inline]
pub fn find_string_special(haystack: &[u8]) -> Option<usize> {
    #[cfg(target_arch = "aarch64")]
    {
        return unsafe { find_string_special_neon(haystack) };
    }
    #[cfg(not(target_arch = "aarch64"))]
    {
        find_string_special_scalar(haystack)
    }
}

/// Index of the first non-JSON-whitespace byte (`space` / `\t` / `\n` / `\r`),
/// or `None` if the whole slice is whitespace.
#[inline]
pub fn find_non_whitespace(haystack: &[u8]) -> Option<usize> {
    #[cfg(target_arch = "aarch64")]
    {
        return unsafe { find_non_whitespace_neon(haystack) };
    }
    #[cfg(not(target_arch = "aarch64"))]
    {
        find_non_whitespace_scalar(haystack)
    }
}

#[cfg(target_arch = "aarch64")]
#[target_feature(enable = "neon")]
unsafe fn find_string_special_neon(haystack: &[u8]) -> Option<usize> {
    use std::arch::aarch64::*;

    let quote = vdupq_n_u8(b'"');
    let backslash = vdupq_n_u8(b'\\');
    let ctrl_max = vdupq_n_u8(0x1F);

    let mut offset = 0usize;
    while offset + 16 <= haystack.len() {
        // SAFETY: `offset + 16 <= len`.
        let block = unsafe { vld1q_u8(haystack.as_ptr().add(offset)) };
        let is_quote = vceqq_u8(block, quote);
        let is_backslash = vceqq_u8(block, backslash);
        // unsigned `b <= 0x1F` ⇔ `min(b, 0x1F) == b`
        let is_control = vceqq_u8(vminq_u8(block, ctrl_max), block);
        let matched = vorrq_u8(is_quote, vorrq_u8(is_backslash, is_control));
        let mask = unsafe { neon_mask16(matched) };
        if mask != 0 {
            return Some(offset + mask.trailing_zeros() as usize);
        }
        offset += 16;
    }

    find_string_special_scalar(&haystack[offset..]).map(|i| offset + i)
}

#[cfg(target_arch = "aarch64")]
#[target_feature(enable = "neon")]
unsafe fn find_non_whitespace_neon(haystack: &[u8]) -> Option<usize> {
    use std::arch::aarch64::*;

    let space = vdupq_n_u8(b' ');
    let tab = vdupq_n_u8(b'\t');
    let lf = vdupq_n_u8(b'\n');
    let cr = vdupq_n_u8(b'\r');

    let mut offset = 0usize;
    while offset + 16 <= haystack.len() {
        let block = unsafe { vld1q_u8(haystack.as_ptr().add(offset)) };
        let is_ws = vorrq_u8(
            vorrq_u8(vceqq_u8(block, space), vceqq_u8(block, tab)),
            vorrq_u8(vceqq_u8(block, lf), vceqq_u8(block, cr)),
        );
        let non_ws = vmvnq_u8(is_ws);
        let mask = unsafe { neon_mask16(non_ws) };
        if mask != 0 {
            return Some(offset + mask.trailing_zeros() as usize);
        }
        offset += 16;
    }

    find_non_whitespace_scalar(&haystack[offset..]).map(|i| offset + i)
}

/// Compact 16 NEON comparison lanes (0x00 / 0xFF) into a 16-bit mask.
#[cfg(target_arch = "aarch64")]
#[target_feature(enable = "neon")]
unsafe fn neon_mask16(lanes: std::arch::aarch64::uint8x16_t) -> u32 {
    use std::arch::aarch64::*;
    let mut bytes = [0u8; 16];
    unsafe { vst1q_u8(bytes.as_mut_ptr(), lanes) };
    let mut mask = 0u32;
    for (i, b) in bytes.iter().enumerate() {
        if *b != 0 {
            mask |= 1 << i;
        }
    }
    mask
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json::simd::{find_non_whitespace_scalar, find_string_special_scalar};

    #[test]
    fn string_special_dispatch_matches_scalar() {
        let mut long = vec![b'a'; 100];
        long.push(b'"');
        let samples: [&[u8]; 7] = [
            b"",
            b"hello",
            b"hello\"",
            b"hel\\lo",
            b"he\x01lo",
            &[b'x'; 40],
            long.as_slice(),
        ];
        for s in samples {
            assert_eq!(
                find_string_special(s),
                find_string_special_scalar(s),
                "mismatch on {s:?}"
            );
        }
    }

    #[test]
    fn non_whitespace_dispatch_matches_scalar() {
        let mut long = vec![b' '; 100];
        long.push(b'[');
        let samples: [&[u8]; 8] = [
            b"",
            b"   ",
            b"\t\n\r ",
            b"  x",
            b"x",
            b"{\n",
            &[b' '; 40],
            long.as_slice(),
        ];
        for s in samples {
            assert_eq!(
                find_non_whitespace(s),
                find_non_whitespace_scalar(s),
                "mismatch on {s:?}"
            );
        }
    }

    #[test]
    fn available_on_aarch64() {
        assert_eq!(is_available(), cfg!(target_arch = "aarch64"));
    }
}
