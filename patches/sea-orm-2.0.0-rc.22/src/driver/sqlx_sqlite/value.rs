//! SQLite value conversion with proper TEXT binding for NaiveDateTime

use sea_orm::sea_query::Value;
use sqlx::sqlite::SqliteArgumentValue;
use sea_orm::FromValueResult;

/// Workaround for SeaORM 2.0.0-rc.22 SQLite NaiveDateTime binding bug
pub fn encode_naive_datetime_as_text(value: Value) -> SqliteArgumentValue<'static> {
    match value {
        Value::DateTime(dt) => {
            // Convert to ISO 8601 string format that SQLite expects
            let dt_str = dt.format("%Y-%m-%d %H:%M:%S%.f").to_string();
            SqliteArgumentValue::Text(dt_str.into())
        }
        Value::DateTimeUtc(dt) => {
            let dt_str = dt.format("%Y-%m-%d %H:%M:%S%.f").to_string();
            SqliteArgumentValue::Text(dt_str.into())
        }
        Value::TimeDate(dt) => {
            let dt_str = dt.format("%Y-%m-%d").to_string();
            SqliteArgumentValue::Text(dt_str.into())
        }
        Value::Time(dt) => {
            let dt_str = dt.format("%H:%M:%S%.f").to_string();
            SqliteArgumentValue::Text(dt_str.into())
        }
        _ => unreachable!("This function is only for datetime types"),
    }
}

/// Parse TEXT back to NaiveDateTime when decoding
pub fn decode_text_to_naive_datetime(value: String) -> FromValueResult<NaiveDateTime> {
    match NaiveDateTime::parse_from_str(&value, "%Y-%m-%d %H:%M:%S%.f") {
        Ok(dt) => FromValueResult::Ok(dt),
        Err(e) => FromValueResult::Err(e.into()),
    }
}