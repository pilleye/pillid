//! A tinier, prefixed, URL-friendly, time-sortable, unique ID storable on the stack.

use std::ffi::c_char;
use std::ffi::CStr;
use std::fmt::Debug;
use std::fmt::Display;
use std::str::FromStr;

use anyhow::Result;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use thiserror::Error;

mod rng;

#[cfg(feature = "sqlx")]
mod db;

/// The maximum length of the prefix.
const PREFIX_BYTES: usize = 32;

/// The number of digits required to represent the timestamp.
const PREFIX_LENGTH: usize = PREFIX_BYTES;

/// The number of bytes to represent the timestamp.
const TIMESTAMP_BYTES: usize = 8;

/// The number of digits required to represent the timestamp.
/// Calculated by \lfloor log_62 (number of bits) + 1 \rfloor
///
/// This actually requires 11 bytes, but with 6 digits, zzzzzz == December 3769,
/// so unless this exists for 2000 years, we're fine.
const TIMESTAMP_LENGTH: usize = 6;

/// Gives us 8 * RANDOMNESS_LENGTH bits of entropy.
/// This is 128 bits of entropy.
const RANDOMNESS_BYTES: usize = 16;

/// The number of digits required to represent the random number.
/// Calculated by \lfloor log_62 (number of bits) + 1 \rfloor
const RANDOM_LENGTH: usize = 22;

/// The characters to use to generate an ID.
const CHARSET: &[u8; 62] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

/// The maximum length of an ID.
const MAX_LENGTH_PILLID: usize = PREFIX_LENGTH + TIMESTAMP_LENGTH + RANDOM_LENGTH;

/// An ID that may be used to identify a resource.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialOrd, PartialEq)]
pub struct Pillid([u8; MAX_LENGTH_PILLID + 1]);

impl Pillid {
    pub fn builder() -> PillidBuilder {
        PillidBuilder::default()
    }

    pub fn new(prefix: &str) -> Self {
        PillidBuilder::new().with_prefix(prefix).unwrap().build()
    }
}

impl Default for Pillid {
    fn default() -> Self {
        unsafe { Pillid(std::mem::zeroed()) }
    }
}

impl Display for Pillid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", str_from_bytes(&self.0))
    }
}

impl Debug for Pillid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl FromStr for Pillid {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.as_bytes().len() >= MAX_LENGTH_PILLID {
            return Err(anyhow::anyhow!("Pillid is too long"));
        }

        let mut bytes: [u8; MAX_LENGTH_PILLID + 1] = unsafe { std::mem::zeroed() };
        bytes[..s.as_bytes().len()].copy_from_slice(s.as_bytes());
        Ok(Pillid(bytes))
    }
}

impl From<String> for Pillid {
    fn from(s: String) -> Self {
        Pillid::from_str(&s).unwrap()
    }
}

impl Serialize for Pillid {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(str_from_bytes(&self.0))
    }
}

impl<'de> Deserialize<'de> for Pillid {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;

        if s.as_bytes().len() > MAX_LENGTH_PILLID {
            return Err(serde::de::Error::custom("Pillid is too long"));
        }

        Ok(Pillid::from(s))
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("prefix must be at most {} characters", PREFIX_LENGTH)]
    PrefixTooLong,
}

/// A unique identifier across the application.
///
/// This contains a ASCII prefix of at most 16 characters, a timestamp to the
/// nearest second, and a CSPRNG-based random number in base62.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialOrd, PartialEq)]
pub struct PillidBuilder {
    prefix: Option<[u8; PREFIX_BYTES]>,
    timestamp: [u8; TIMESTAMP_BYTES],
    random: [u8; RANDOMNESS_BYTES],
}

impl Default for PillidBuilder {
    fn default() -> Self {
        PillidBuilder {
            prefix: None,
            timestamp: [0u8; TIMESTAMP_BYTES],
            random: [0u8; RANDOMNESS_BYTES],
        }
    }
}

impl Display for PillidBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut timestamp_bytes = [0; TIMESTAMP_LENGTH + 1];
        let mut random_bytes = [0; RANDOM_LENGTH + 1];

        fn u128_to_base62_str(n: u128, len: usize, output_buffer: &mut [u8]) {
            let mut n = n;

            for i in (0..len).rev() {
                output_buffer[i] = *CHARSET.get((n % 62) as usize).unwrap();
                n /= 62;
            }
        }

        u128_to_base62_str(
            u64::from_be_bytes(*self.timestamp()).into(),
            TIMESTAMP_LENGTH,
            &mut timestamp_bytes,
        );

        u128_to_base62_str(
            u128::from_be_bytes(*self.random()),
            RANDOM_LENGTH,
            &mut random_bytes,
        );

        if let Some(prefix) = self.prefix {
            write!(f, "{}_", str_from_bytes(&prefix))?;
        }

        write!(
            f,
            "{}{}",
            str_from_bytes(&timestamp_bytes),
            str_from_bytes(&random_bytes)
        )
    }
}

impl PillidBuilder {
    pub(crate) fn new() -> Self {
        let timestamp = (Utc::now().timestamp() as u64).to_be_bytes();

        PillidBuilder::default()
            .with_timestamp(timestamp)
            .with_random(rng::bytes())
    }

    pub fn random(&self) -> &[u8; RANDOMNESS_BYTES] {
        &self.random
    }

    pub fn set_random(&mut self, random: [u8; RANDOMNESS_BYTES]) {
        self.random = random
    }

    pub fn with_random(self, random: [u8; RANDOMNESS_BYTES]) -> Self {
        let mut id = self;
        id.set_random(random);
        id
    }

    pub fn timestamp(&self) -> &[u8; TIMESTAMP_BYTES] {
        &self.timestamp
    }

    pub fn set_timestamp(&mut self, timestamp: [u8; TIMESTAMP_BYTES]) {
        self.timestamp = timestamp;
    }

    pub fn with_timestamp(self, timestamp: [u8; TIMESTAMP_BYTES]) -> Self {
        let mut id = self;
        id.set_timestamp(timestamp);
        id
    }

    pub fn prefix(&self) -> Option<&str> {
        self.prefix.as_ref().map(|p| str_from_bytes(p))
    }

    pub fn set_prefix(&mut self, prefix: &str) -> Result<(), Error> {
        if prefix.as_bytes().len() > PREFIX_LENGTH {
            return Err(Error::PrefixTooLong);
        }

        let mut new_prefix = [0u8; 32];
        new_prefix[..prefix.as_bytes().len()].copy_from_slice(prefix.as_bytes());
        self.prefix = Some(new_prefix);

        Ok(())
    }

    pub fn with_prefix(self, prefix: &str) -> Result<Self, Error> {
        let mut id = self;
        id.set_prefix(prefix)?;
        Ok(id)
    }

    pub fn build(self) -> Pillid {
        let mut output_bytes = [0u8; MAX_LENGTH_PILLID + 1];
        let output_str = self.to_string();
        output_bytes[..output_str.len()].copy_from_slice(output_str.as_bytes());
        Pillid(output_bytes)
    }
}

fn str_from_bytes(bytes: &[u8]) -> &str {
    unsafe { CStr::from_ptr(bytes.as_ptr() as *const c_char) }
        .to_str()
        .unwrap()
}

#[macro_export]
macro_rules! pillid {
    ($t:ident, $prefix:expr) => {
        paste::paste! {
            #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
            pub struct [<$t Pillid>]($crate::Pillid);

            impl [<$t Pillid>] {
                pub fn new() -> Self {
                    Self($crate::Pillid::new($prefix))
                }
            }

            impl Default for [<$t Pillid>] {
                fn default() -> Self {
                    Self::new()
                }
            }

            impl std::fmt::Display for [<$t Pillid>] {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    write!(f, "{}", self.0)
                }
            }

            impl std::fmt::Debug for [<$t Pillid>] {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    write!(f, "{:?}", self.0)
                }
            }

            impl std::str::FromStr for [<$t Pillid>] {
                type Err = anyhow::Error;

                fn from_str(s: &str) -> Result<Self, Self::Err> {
                    Ok(Self($crate::Pillid::from_str(s)?))
                }
            }

            impl From<String> for [<$t Pillid>] {
                fn from(s: String) -> Self {
                    use std::str::FromStr;
                    Self::from_str(&s).unwrap()
                }
            }

            impl std::convert::From<[<$t Pillid>]> for $crate::Pillid {
                fn from(specialized_pillid: [<$t Pillid>]) -> $crate::Pillid {
                    specialized_pillid.0
                }
            }

            impl std::convert::From<$crate::Pillid> for [<$t Pillid>] {
                fn from(pillid: $crate::Pillid) -> [<$t Pillid>] {
                    [<$t Pillid>](pillid)
                }
            }


            impl Serialize for [<$t Pillid>] {
                fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                    self.0.serialize(serializer)
                }
            }

            impl<'de> Deserialize<'de> for [<$t Pillid>] {
                fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
                where
                    D: serde::Deserializer<'de>,
                {
                    $crate::Pillid::deserialize(deserializer).map(Into::into)
                }
            }

            #[cfg(feature = "sqlx")]
            impl<'q> sqlx::Encode<'q, sqlx::Sqlite> for [<$t Pillid>] {
                fn encode(self, args: &mut Vec<sqlx::sqlite::SqliteArgumentValue<'q>>) -> sqlx::encode::IsNull {
                    self.0.encode(args)
                }

                fn encode_by_ref(&self, args: &mut Vec<sqlx::sqlite::SqliteArgumentValue<'q>>) -> sqlx::encode::IsNull {
                    self.0.encode_by_ref(args)
                }
            }


            #[cfg(feature = "sqlx")]
            impl sqlx::Type<sqlx::Sqlite> for [<$t Pillid>] {
                fn type_info() -> sqlx::sqlite::SqliteTypeInfo {
                    <&str as sqlx::Type<sqlx::Sqlite>>::type_info()
                }
            }

        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default() -> Result<()> {
        let pillid = PillidBuilder::default();
        assert_eq!(pillid.prefix(), None);
        assert_eq!(pillid.timestamp(), &[0x00; TIMESTAMP_BYTES]);
        assert_eq!(pillid.random(), &[0x00; RANDOMNESS_BYTES]);
        Ok(())
    }

    #[test]
    fn test_with_prefix() -> Result<()> {
        let pillid = PillidBuilder::default().with_prefix("test")?;
        assert_eq!(pillid.prefix(), Some("test"));
        assert_eq!(pillid.timestamp(), &[0x00; TIMESTAMP_BYTES]);
        assert_eq!(pillid.random(), &[0x00; RANDOMNESS_BYTES]);
        Ok(())
    }

    #[test]
    fn test_with_prefix_too_long() -> Result<()> {
        // 32 long
        PillidBuilder::default().with_prefix("12345678901234567890123456789012")?;

        // 33 long
        assert!(PillidBuilder::default()
            .with_prefix("123456789012345678901234567890123")
            .is_err());

        Ok(())
    }

    #[test]
    fn test_with_timestamp() -> Result<()> {
        let pillid = PillidBuilder::default().with_timestamp([0xFF; TIMESTAMP_BYTES]);
        assert_eq!(pillid.prefix(), None);
        assert_eq!(pillid.timestamp(), &[0xFF; TIMESTAMP_BYTES]);
        assert_eq!(pillid.random(), &[0x00; RANDOMNESS_BYTES]);

        Ok(())
    }

    #[test]
    fn test_with_random() -> Result<()> {
        let pillid = PillidBuilder::default().with_random([0xFF; RANDOMNESS_BYTES]);
        assert_eq!(pillid.prefix(), None);
        assert_eq!(pillid.timestamp(), &[0x00; TIMESTAMP_BYTES]);
        assert_eq!(pillid.random(), &[0xFF; RANDOMNESS_BYTES]);

        Ok(())
    }

    #[test]
    fn test_display() -> Result<()> {
        let pillid = PillidBuilder::default()
            .with_prefix("prefixed")?
            .with_timestamp([0xFF; TIMESTAMP_BYTES])
            .with_random([0xFF; RANDOMNESS_BYTES])
            .build();

        assert_eq!(pillid.to_string(), "prefixed_16AHYF7n42DGM5Tflk9n8mt7Fhc7");

        Ok(())
    }

    pillid!(Foo, "foo");

    #[test]
    fn test_custom_pillid() -> Result<()> {
        let pillid = FooPillid::new();
        assert!(pillid.to_string().starts_with("foo_"));
        Ok(())
    }

    pillid!(Bar, String::from("bar").as_str());

    #[test]
    fn test_non_literal_custom_pillid() -> Result<()> {
        let pillid = BarPillid::new();
        assert!(pillid.to_string().starts_with("bar_"));
        Ok(())
    }
}
