use sqlx::encode::IsNull;
use sqlx::sqlite::SqliteArgumentValue;
use sqlx::sqlite::SqliteTypeInfo;
use sqlx::Encode;
use sqlx::Sqlite;
use sqlx::Type;

use super::Pillid;

impl<'q> Encode<'q, Sqlite> for Pillid {
    fn encode(self, args: &mut Vec<SqliteArgumentValue<'q>>) -> IsNull {
        <String as Encode<Sqlite>>::encode(self.to_string(), args)
    }

    fn encode_by_ref(&self, args: &mut Vec<SqliteArgumentValue<'q>>) -> IsNull {
        <String as Encode<Sqlite>>::encode(self.to_string(), args)
    }
}

impl Type<Sqlite> for Pillid {
    fn type_info() -> SqliteTypeInfo {
        <&str as Type<Sqlite>>::type_info()
    }
}
