use std::fmt;
use std::marker::PhantomData;
use std::str::FromStr;
use std::time::SystemTime;

pub mod alphabet;
pub mod rngs;
mod timestamp;

#[doc(hidden)]
pub use paste;

/// Number of base62 characters reserved for the timestamp in timestamped IDs.
pub const TS_SIZE: usize = 8;

/// Compile-time prefix attached to a [`Pillid`].
///
/// Implement this trait directly or use the [`prefix!`] macro.
pub trait Prefix {
    const VALUE: &'static str;
    const TIMESTAMPED: bool = false;
}

/// A typed, `Copy`-able, stack-allocated prefixed ID.
///
/// The inner storage is `[u8; N]` ASCII base62 characters (no prefix).
/// Use the [`prefix!`] macro to define concrete ID types.
///
/// # Example
///
/// ```rust
/// use pillid::prefix;
///
/// prefix!(pub UserId, "usr");
///
/// let id = UserId::new();
/// let s = id.to_string();        // e.g. "usr_cNbQxzR55W2RbkPoERACA"
/// let parsed: UserId = s.parse().unwrap();
/// assert_eq!(id, parsed);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Pillid<T: Prefix, const N: usize = 22> {
    inner: [u8; N],
    _marker: PhantomData<T>,
}

impl<T: Prefix, const N: usize> Pillid<T, N> {
    /// Generate a new random ID.
    ///
    /// For timestamped prefix types (defined with `ts` in [`prefix!`]), the first
    /// [`TS_SIZE`] characters are a fixed-width base62-encoded Unix timestamp,
    /// making IDs time-sortable. The remainder is random.
    pub fn new() -> Self {
        // Evaluated at compile time — catches misconfigured N at monomorphization.
        const {
            assert!(
                !T::TIMESTAMPED || N >= TS_SIZE,
                "Pillid: N must be >= TS_SIZE (8) for timestamped prefix types"
            )
        };

        let mut inner = [0u8; N];

        let rand_start = if T::TIMESTAMPED {
            let ts = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            inner[..TS_SIZE].copy_from_slice(&timestamp::encode_timestamp_8(ts));
            TS_SIZE
        } else {
            0
        };

        fill_base62(&mut inner[rand_start..]);

        Self {
            inner,
            _marker: PhantomData,
        }
    }

    /// The compile-time prefix string for this ID type.
    pub const fn prefix() -> &'static str {
        T::VALUE
    }

    /// The inner ID characters without the prefix or separator.
    ///
    /// Trailing null bytes (from parsing a shorter legacy ID) are stripped,
    /// so this returns the original string at its original length.
    pub fn as_str(&self) -> &str {
        let len = N - self.inner.iter().rev().take_while(|&&b| b == 0).count();
        // Safety: inner contains valid ASCII base62 chars or null bytes (both valid UTF-8).
        unsafe { std::str::from_utf8_unchecked(&self.inner[..len]) }
    }
}

impl<T: Prefix, const N: usize> Default for Pillid<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Prefix, const N: usize> fmt::Display for Pillid<T, N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}_{}", T::VALUE, self.as_str())
    }
}

impl<T: Prefix, const N: usize> FromStr for Pillid<T, N> {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let expected_prefix = format!("{}_", T::VALUE);
        let id_str = s.strip_prefix(&expected_prefix).ok_or_else(|| {
            let found_prefix = s.rsplit_once('_').map_or(s, |(prefix, _)| prefix);
            format!(
                "wrong prefix: expected '{}', found '{}'",
                T::VALUE,
                found_prefix
            )
        })?;

        if id_str.len() > N {
            return Err(format!(
                "wrong length: expected at most {N}, found {}",
                id_str.len()
            ));
        }

        // Shorter legacy IDs are right-padded with null bytes. as_str() strips them.
        let mut inner = [0u8; N];
        inner[..id_str.len()].copy_from_slice(id_str.as_bytes());
        Ok(Self {
            inner,
            _marker: PhantomData,
        })
    }
}

impl<T: Prefix, const N: usize> From<String> for Pillid<T, N> {
    fn from(s: String) -> Self {
        s.parse().expect("invalid Pillid in database")
    }
}

fn fill_base62(buf: &mut [u8]) {
    let size = buf.len();
    if size == 0 {
        return;
    }
    let mask = alphabet::DEFAULT.len().next_power_of_two() - 1;
    let step = 2 * size * mask;
    let mut filled = 0;
    loop {
        let bytes = rngs::default(step);
        for &byte in &bytes {
            let idx = byte as usize & mask;
            if idx < alphabet::DEFAULT.len() {
                buf[filled] = alphabet::DEFAULT[idx] as u8;
                filled += 1;
                if filled == size {
                    return;
                }
            }
        }
    }
}

/// Define a typed prefix and generate a [`Pillid`] type alias.
///
/// # Variants
///
/// ```text
/// prefix!(pub UserId, "usr");             // 22 random chars, no timestamp
/// prefix!(pub ShortId, "sid", 10);        // 10 random chars, no timestamp
/// prefix!(pub EventId, "evt", ts);        // 8 ts + 22 random = 30 total chars
/// prefix!(pub SmallEvent, "sev", 10, ts); // 8 ts + 10 random = 18 total chars
/// ```
///
/// The generated type alias is `Copy`, `Eq`, `Ord`, `Hash`, and type-safe:
/// a `UserId` cannot be passed where a `SessionId` is expected.
///
/// Compound prefixes (containing `_`) work as expected:
///
/// ```rust
/// use pillid::prefix;
///
/// prefix!(pub CompoundId, "rsy_vn");
///
/// let id = CompoundId::new();
/// assert!(id.to_string().starts_with("rsy_vn_"));
/// assert_eq!(id.to_string().parse(), Ok(id));
/// ```
#[macro_export]
macro_rules! prefix {
    ($vis:vis $name:ident, $prefix:literal) => {
        $crate::prefix!($vis $name, $prefix, 22);
    };

    ($vis:vis $name:ident, $prefix:literal, $size:literal) => {
        $crate::paste::paste! {
            #[doc(hidden)]
            #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
            $vis struct [<__ $name Prefix>];

            impl $crate::Prefix for [<__ $name Prefix>] {
                const VALUE: &'static str = $prefix;
                const TIMESTAMPED: bool = false;
            }

            $vis type $name = $crate::Pillid<[<__ $name Prefix>], $size>;
        }
    };

    ($vis:vis $name:ident, $prefix:literal, ts) => {
        $crate::prefix!($vis $name, $prefix, 22, ts);
    };

    ($vis:vis $name:ident, $prefix:literal, $size:literal, ts) => {
        $crate::paste::paste! {
            #[doc(hidden)]
            #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
            $vis struct [<__ $name Prefix>];

            impl $crate::Prefix for [<__ $name Prefix>] {
                const VALUE: &'static str = $prefix;
                const TIMESTAMPED: bool = true;
            }

            // Named const avoids const-expression arithmetic in type-alias position.
            #[doc(hidden)]
            #[allow(non_upper_case_globals)]
            const [<__ $name _PILLID_N>]: usize = $crate::TS_SIZE + $size;

            $vis type $name = $crate::Pillid<[<__ $name Prefix>], [<__ $name _PILLID_N>]>;
        }
    };
}

#[cfg(feature = "serde")]
mod serde_impl {
    use std::str::FromStr;

    use serde::de;
    use serde::Deserialize;
    use serde::Deserializer;
    use serde::Serialize;
    use serde::Serializer;

    use super::*;

    impl<T: Prefix, const N: usize> Serialize for Pillid<T, N> {
        fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
            s.serialize_str(&self.to_string())
        }
    }

    impl<'de, T: Prefix, const N: usize> Deserialize<'de> for Pillid<T, N> {
        fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
            let s = String::deserialize(d)?;
            Self::from_str(&s).map_err(de::Error::custom)
        }
    }
}

#[cfg(feature = "sqlx")]
mod sqlx_impl {
    use sqlx::encode::Encode;
    use sqlx::encode::IsNull;
    use sqlx::error::BoxDynError;
    use sqlx::Database;
    use sqlx::Decode;
    use sqlx::Type;

    use super::*;

    impl<T: Prefix, const N: usize, DB: Database> Type<DB> for Pillid<T, N>
    where
        str: Type<DB>,
    {
        fn type_info() -> DB::TypeInfo {
            <str as Type<DB>>::type_info()
        }

        fn compatible(ty: &DB::TypeInfo) -> bool {
            <str as Type<DB>>::compatible(ty)
        }
    }

    impl<'r, T: Prefix, const N: usize, DB: Database> Decode<'r, DB> for Pillid<T, N>
    where
        &'r str: Decode<'r, DB>,
    {
        fn decode(value: DB::ValueRef<'r>) -> Result<Self, BoxDynError> {
            let s = <&str as Decode<DB>>::decode(value)?;
            Self::from_str(s).map_err(|e| -> BoxDynError { e.into() })
        }
    }

    impl<'q, T: Prefix, const N: usize, DB: Database> Encode<'q, DB> for Pillid<T, N>
    where
        String: Encode<'q, DB>,
    {
        fn encode_by_ref(&self, buf: &mut DB::ArgumentBuffer) -> Result<IsNull, BoxDynError> {
            String::encode(self.to_string(), buf)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    prefix!(TestUser, "usr");
    prefix!(TestSession, "ses");
    prefix!(TestShort, "shrt", 10);
    prefix!(TestEvent, "evt", ts);
    prefix!(TestSmallEvent, "sev", 10, ts);
    prefix!(TestCompound, "rsy_vn");

    #[test]
    fn new_produces_correct_length() {
        let id = TestUser::new();
        assert_eq!(id.as_str().len(), 22);
    }

    #[test]
    fn custom_size() {
        let id = TestShort::new();
        assert_eq!(id.as_str().len(), 10);
    }

    #[test]
    fn timestamped_has_ts_plus_random_length() {
        let id = TestEvent::new();
        assert_eq!(id.as_str().len(), TS_SIZE + 22);
    }

    #[test]
    fn timestamped_small() {
        let id = TestSmallEvent::new();
        assert_eq!(id.as_str().len(), TS_SIZE + 10);
    }

    #[test]
    fn display_format() {
        let id = TestUser::new();
        let s = id.to_string();
        assert!(s.starts_with("usr_"));
        assert_eq!(s.len(), "usr_".len() + 22);
    }

    #[test]
    fn round_trips() {
        let id = TestUser::new();
        assert_eq!(id.to_string().parse(), Ok(id));
    }

    #[test]
    fn round_trips_short() {
        let id = TestShort::new();
        assert_eq!(id.to_string().parse(), Ok(id));
    }

    #[test]
    fn round_trips_timestamped() {
        let id = TestEvent::new();
        assert_eq!(id.to_string().parse(), Ok(id));
    }

    #[test]
    fn round_trips_compound_prefix() {
        let id = TestCompound::new();
        assert_eq!(id.to_string().parse(), Ok(id));
    }

    #[test]
    fn rejects_wrong_prefix() {
        let err = "ses_cNbQxzR55W2RbkPoERACA"
            .parse::<TestUser>()
            .expect_err("wrong prefix must be rejected");
        assert_eq!(err, "wrong prefix: expected 'usr', found 'ses'");
    }

    #[test]
    fn rejects_wrong_compound_prefix() {
        let err = "rsy_ds_cNbQxzR55W2RbkPoERACA"
            .parse::<TestCompound>()
            .expect_err("wrong compound prefix must be rejected");
        assert_eq!(err, "wrong prefix: expected 'rsy_vn', found 'rsy_ds'");
    }

    #[test]
    fn rejects_longer_than_n() {
        // 23-char inner into a 22-char type — no truncation supported
        let err = "usr_cNbQxzR55W2RbkPoERACAxy"
            .parse::<TestUser>()
            .expect_err("longer ID must be rejected");
        assert!(err.contains("wrong length"));
    }

    #[test]
    fn accepts_shorter_legacy_id() {
        // Simulate reading an old N=10 ID into an expanded N=14 type.
        prefix!(ExpandedUser, "usr", 14);
        let old = "usr_cNbQxzR55W"; // 10-char inner
        let id: ExpandedUser = old.parse().expect("legacy ID should parse");
        assert_eq!(id.to_string(), old); // display at original length
        assert_eq!(id.to_string().parse(), Ok(id)); // round-trips
    }

    #[test]
    fn is_copy() {
        let id = TestUser::new();
        let a = id;
        let b = id;
        assert_eq!(a, b);
    }

    #[test]
    fn different_types_are_distinct() {
        // This test verifies type-safety at compile time — the two types are
        // incompatible even though both are 22-char IDs with the same N.
        let _user: TestUser = TestUser::new();
        let _session: TestSession = TestSession::new();
        // The following would be a compile error:
        // let _: TestUser = _session;
    }
}
