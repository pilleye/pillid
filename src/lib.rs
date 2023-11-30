//! A tiny, secure, URL-friendly, prefixed (optional), timestamped (optional),
//! unique string ID generator
//!
//! **Safe.** It uses cryptographically strong random APIs
//! and guarantees a proper distribution of symbols.
//!
//! **Compact.** It uses a larger alphabet than UUID (`A-Za-z0-9`)
//! and has more unique IDs in just 22 symbols instead of 36.
//!
//! ```toml
//! [dependencies]
//! pillid = "0.4.0"
//! ```
//!
//! ```rust
//! use pillid::pillid;
//!
//! fn main() {
//!    let id = pillid!(); //=> "cNbQxzR55W2RbkPoERACA"
//! }
//! ```
//!
//! ## Usage
//!
//! ### Simple
//!
//! The main module uses URL-friendly symbols (`A-Za-z0-9`) and returns an ID
//! with 22 characters (equivalent to 128 random bits).
//!
//! ```rust
//! use pillid::pillid;
//!
//! fn main() {
//!    let id = pillid!(); //=> "cNbQxzR55W2RbkPoERACA"
//! }
//! ```
//ghnfvhtkbjvibclbikuitg!
//! ### Custom length
//!
//! If you want to reduce ID length (and increase collisions probability),
//! you can pass the length as an argument generate function:
//!
//! ```rust
//! use pillid::pillid;
//!
//! fn main() {
//!    let id = pillid!(10); //=> "QhpvygNybI"
//! }
//! ```
//!
//! ### Custom Alphabet or Length
//!
//! If you want to change the ID's alphabet or length
//! you can use the low-level `custom` module.
//!
//! ```rust
//! use pillid::pillid;
//!
//! fn main() {
//!     let alphabet: [char; 16] = [
//!         '1', '2', '3', '4', '5', '6', '7', '8', '9', '0', 'a', 'b', 'c', 'd', 'e', 'f'
//!     ];
//!
//!    let id = pillid!(10, &alphabet); //=> "f42ega7402"
//! }
//! ```
//!
//! Alphabet must contain 256 symbols or less.
//! Otherwise, the generator will not be secure.
//!
//! ### Custom Random Bytes Generator
//!
//! You can replace the default safe random generator using the `complex` module.
//! For instance, to use a seed-based generator.
//!
//! ```rust
//! use pillid::pillid;
//!
//! fn random_byte () -> u8 {
//!     0
//! }
//!
//! fn main() {
//!     fn random (size: usize) -> Vec<u8> {
//!         let mut bytes: Vec<u8> = vec![0; size];
//!
//!         for i in 0..size {
//!             bytes[i] = random_byte();
//!         }
//!
//!         bytes
//!     }
//!
//!     pillid!(10, &['a', 'b', 'c', 'd', 'e', 'f'], random); //=> "fbaefaadeb"
//! }
//! ```
//!
//! `random` function must accept the array size and return an vector
//! with random numbers.
//!
//! If you want to use the same URL-friendly symbols with `format`,
//! you can get the default alphabet from the `url` module:
//!
//! ```rust
//! use pillid::pillid;
//!
//! fn random (size: usize) -> Vec<u8> {
//!     let result: Vec<u8> = vec![0; size];
//!
//!     result
//! }
//!
//! fn main() {
//!     pillid!(10, &pillid::alphabet::DEFAULT, random); //=> "93celLtuub"
//! }
//! ```
//!

#![doc(
    html_logo_url = "https://www.rust-lang.org/logos/rust-logo-128x128-blk.png",
    html_favicon_url = "https://www.rust-lang.org/favicon.ico",
    html_root_url = "https://docs.rs/pillid"
)]

use std::time::SystemTime;

#[cfg(feature = "smartstring")]
use smartstring::alias::String;

pub mod alphabet;
pub mod rngs;
mod timestamp;
mod utils;

/// Struct to hold the configuration for an arbitrary ID generator.
///
/// # Example
///
/// ```
/// use pillid::PillidGenerator;
///
/// let generator = PillidGenerator::new().with_prefix("pre".into()).with_timestamp();
/// let id = generator.generate(5, &['0', '1', '2'], |_| vec![0; 10]);
/// //=> "pre_{timestamp}00000"
/// assert!(id.starts_with("pre_"));
/// assert!(id.ends_with("00000"));
/// assert_ne!(id, "pre_00000");
/// ```
pub struct PillidGenerator {
    prefix: Option<String>,
    timestamp: bool,
}

impl PillidGenerator {
    pub fn new() -> Self {
        PillidGenerator {
            prefix: None,
            timestamp: false,
        }
    }

    pub fn with_prefix(mut self, prefix: String) -> Self {
        self.prefix = Some(prefix);
        self
    }

    pub fn with_timestamp(mut self) -> Self {
        self.timestamp = true;
        self
    }

    pub fn generate(
        self,
        size: usize,
        alphabet: &[char],
        random: impl Fn(usize) -> Vec<u8>,
    ) -> String {
        generate(
            size,
            alphabet,
            self.prefix,
            self.timestamp.then(|| {
                SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
            }),
            random,
        )
    }
}

#[cfg(test)]
mod test_config {
    use super::*;

    #[test]
    fn generates_random_string() {
        fn random(size: usize) -> Vec<u8> {
            [2, 255, 0, 1].iter().cloned().cycle().take(size).collect()
        }

        assert_eq!(generate(4, &['0', '1', '2'], None, None, random), "2012");
    }

    #[test]
    fn generates_random_string_with_timestamp() {
        fn random(size: usize) -> Vec<u8> {
            [2, 255, 0, 1].iter().cloned().cycle().take(size).collect()
        }

        let current_time = 15u64;
        let id = generate(4, &['0', '1', '2'], None, Some(current_time), random);

        assert_eq!(id, "1202012");

        let three_thousand_ce = 32503708800u64;
        let id = generate(4, &['0', '1', '2'], None, Some(three_thousand_ce), random);

        assert_eq!(id, "100022200201101110112002012");
    }

    #[test]
    #[should_panic]
    fn bad_alphabet() {
        let alphabet: Vec<char> = (0..32_u8).cycle().map(|i| i as char).take(1000).collect();
        pillid!(22, &alphabet);
    }

    #[test]
    fn non_power_2() {
        let id: String = pillid!(42, &alphabet::DEFAULT);
        assert_eq!(id.len(), 42);
    }
}

pub fn generate(
    size: usize,
    alphabet: &[char],
    prefix: Option<String>,
    timestamp: Option<u64>,
    random: impl Fn(usize) -> Vec<u8>,
) -> String {
    debug_assert!(
        alphabet.len() <= u8::max_value() as usize,
        "The alphabet cannot be longer than a `u8` (to comply with the `random` function)"
    );

    let mask = alphabet.len().next_power_of_two() - 1;
    let step: usize = 2 * size * mask;

    let mut ts = None;
    if let Some(timestamp) = timestamp {
        ts = Some(timestamp::u64_to_string(timestamp, alphabet));
    }

    #[cfg(not(feature = "smartstring"))]
    let mut id = String::with_capacity(utils::string_size(size, &prefix, &ts));
    #[cfg(feature = "smartstring")]
    let mut id = String::new();

    if let Some(prefix) = prefix {
        id.push_str(&prefix);
        id.push('_');
    }

    if let Some(ts) = ts {
        id.push_str(&ts);
    }

    let mut added_chars = 0;
    loop {
        let bytes = (random)(step);

        for &byte in &bytes {
            let byte = byte as usize & mask;

            if alphabet.len() > byte {
                id.push(alphabet[byte]);
                added_chars += 1;

                if added_chars == size {
                    return id;
                }
            }
        }
    }
}

#[cfg(test)]
mod test_format {
    use super::*;

    #[test]
    fn generates_random_string_with_prefix() {
        fn random(size: usize) -> Vec<u8> {
            [2, 255, 0, 1].iter().cloned().cycle().take(size).collect()
        }

        let id =
            PillidGenerator::new()
                .with_prefix("pre".into())
                .generate(4, &['0', '1', '2'], random);

        assert_eq!(id, "pre_2012");
    }

    #[test]
    fn generates_random_string_with_timestamp() {
        fn random(size: usize) -> Vec<u8> {
            [2, 255, 0, 1].iter().cloned().cycle().take(size).collect()
        }

        let id = PillidGenerator::new()
            .with_prefix("pre".into())
            .with_timestamp()
            .generate(4, &['0', '1', '2'], random);

        assert!(id.starts_with("pre_"));
        assert!(id.ends_with("2012"));
        assert_ne!(id, "pre_2012");
    }
}

#[macro_export]
macro_rules! pillid {
    // simple
    () => {
        $crate::generate(
            22,
            &$crate::alphabet::DEFAULT,
            None,
            None,
            $crate::rngs::default,
        )
    };

    // generate
    ($size:expr) => {
        $crate::generate(
            $size,
            &$crate::alphabet::DEFAULT,
            None,
            None,
            $crate::rngs::default,
        )
    };

    // custom
    ($size:expr, $alphabet:expr) => {
        $crate::generate($size, $alphabet, None, None, $crate::rngs::default)
    };

    // complex
    ($size:expr, $alphabet:expr, $random:expr) => {
        $crate::generate($size, $alphabet, None, None, $random)
    };
}

#[cfg(test)]
mod test_macros {
    use super::*;

    #[test]
    fn simple() {
        let id: String = pillid!();
        assert_eq!(id.len(), 22);
    }

    #[test]
    fn generate() {
        let id: String = pillid!(42);

        assert_eq!(id.len(), 42);
    }

    #[test]
    fn custom() {
        let id: String = pillid!(42, &alphabet::URLSAFE);

        assert_eq!(id.len(), 42);
    }

    #[test]
    fn complex() {
        let id: String = pillid!(4, &alphabet::URLSAFE, rngs::default);

        assert_eq!(id.len(), 4);
    }

    #[test]
    fn closure() {
        let uuid = "8936ad0c-9443-4007-9430-e223c64d4629";

        let id1 = pillid!(22, &alphabet::DEFAULT, |_| uuid.as_bytes().to_vec());
        let id2 = pillid!(22, &alphabet::DEFAULT, |_| uuid.as_bytes().to_vec());

        assert_eq!(id1, id2);
    }

    #[test]
    fn simple_expression() {
        let id: String = pillid!(44 / 2);

        assert_eq!(id.len(), 22);
    }
}

#[cfg(doctest)]
doc_comment::doctest!("../README.md");
