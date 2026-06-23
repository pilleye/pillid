use crate::alphabet;

/// Encodes a Unix timestamp as exactly 8 base62 characters (left-padded with '0').
/// 8 base62 chars covers ~218 trillion values — sufficient past year 31000.
pub(crate) fn encode_timestamp_8(mut value: u64) -> [u8; 8] {
    let mut result = [alphabet::DEFAULT[0] as u8; 8];
    for i in (0..8).rev() {
        result[i] = alphabet::DEFAULT[(value % 62) as usize] as u8;
        value /= 62;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_zero_as_eight_zeros() {
        assert_eq!(&encode_timestamp_8(0), b"00000000");
    }

    #[test]
    fn encodes_known_value() {
        // 62^8 - 1 = max value that fits in 8 chars → should be "ZZZZZZZZ"
        let max = 62u64.pow(8) - 1;
        assert_eq!(&encode_timestamp_8(max), b"ZZZZZZZZ");
    }

    #[test]
    fn output_is_always_8_bytes() {
        for ts in [0u64, 1, 62, 3844, 1_000_000, u64::MAX / 2] {
            assert_eq!(encode_timestamp_8(ts).len(), 8);
        }
    }
}
