#[cfg(feature = "sqlx")]
#[doc(hidden)]
#[macro_export]
macro_rules! sqlx_implementations {
    ($t:ident) => {
        paste::paste! {
            impl<'q> sqlx::Encode<'q, sqlx::Sqlite> for [<$t Pillid>] {
                fn encode(self, args: &mut Vec<sqlx::sqlite::SqliteArgumentValue<'q>>) -> sqlx::encode::IsNull {
                    self.0.encode(args)
                }

                fn encode_by_ref(&self, args: &mut Vec<sqlx::sqlite::SqliteArgumentValue<'q>>) -> sqlx::encode::IsNull {
                    self.0.encode_by_ref(args)
                }
            }

            impl sqlx::Type<sqlx::Sqlite> for [<$t Pillid>] {
                fn type_info() -> sqlx::sqlite::SqliteTypeInfo {
                    <&str as sqlx::Type<sqlx::Sqlite>>::type_info()
                }
            }
        }
    }
}

#[cfg(not(feature = "sqlx"))]
#[doc(hidden)]
#[macro_export]
macro_rules! sqlx_implementations {
    ($t:ident) => {};
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

            $crate::sqlx_implementations!($t);
        }
    };
}
