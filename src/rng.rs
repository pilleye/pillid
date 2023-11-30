use rand::rngs::OsRng;
use rand::RngCore;

use super::RANDOMNESS_BYTES;

pub(crate) fn bytes() -> [u8; RANDOMNESS_BYTES] {
    let mut bytes = [0; RANDOMNESS_BYTES];
    OsRng.fill_bytes(&mut bytes);
    bytes
}

#[cfg(test)]
// So this could technically fail, but if a test is flaky when it has a one in
// 2^128 chance of happening, I'll gladly take those odds.
mod tests {
    use super::*;

    #[test]
    fn test_bytes() {
        let bytes1 = bytes();
        let bytes2 = bytes();
        assert_ne!(bytes1, bytes2);
    }

    #[test]
    fn test_not_zero() {
        let bytes = bytes();
        assert_ne!(bytes, [0; RANDOMNESS_BYTES]);
    }
}
