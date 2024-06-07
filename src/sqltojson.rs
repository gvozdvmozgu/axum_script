// https://github.com/lovasoa/SQLpage/blob/main/src/webserver/database/sql_to_json.rs#L43-L91
// linked https://github.com/launchbadge/sqlx/issues/182#issuecomment-1831558170
// Copyright (c) 2023 Ophir LOJKINE

use chrono::{DateTime, Utc};
use serde_json::{self, Map, Value};
use sqlx::any::AnyRow;
use sqlx::Decode;
use sqlx::{postgres::any::AnyColumn, Column, Row, TypeInfo, ValueRef};

pub fn add_value_to_map(
    mut map: Map<String, Value>,
    (key, value): (String, Value),
) -> Map<String, Value> {
    use serde_json::map::Entry::{Occupied, Vacant};
    use Value::Array;
    match map.entry(key) {
        Vacant(vacant) => {
            vacant.insert(value);
        }
        Occupied(mut old_entry) => {
            let mut new_array = if let Array(v) = value { v } else { vec![value] };
            match old_entry.get_mut() {
                Array(old_array) => old_array.append(&mut new_array),
                old_scalar => {
                    new_array.insert(0, old_scalar.take());
                    *old_scalar = Array(new_array);
                }
            }
        }
    }
    map
}

pub fn row_to_json(row: &AnyRow) -> Value {
    use Value::Object;

    let columns = row.columns();
    let mut map = Map::new();
    for col in columns {
        let key = col.name().to_string();
        let value: Value = sql_to_json(row, col);
        map = add_value_to_map(map, (key, value));
    }
    Object(map)
}

pub fn sql_to_json(row: &AnyRow, col: &AnyColumn) -> Value {
    let raw_value_result = row.try_get_raw(col.ordinal());
    match raw_value_result {
        Ok(raw_value) if !raw_value.is_null() => {
            let mut raw_value = Some(raw_value);
            let decoded = sql_nonnull_to_json(|| {
                raw_value
                    .take()
                    .unwrap_or_else(|| row.try_get_raw(col.ordinal()).unwrap())
            });
            decoded
        }
        Ok(_null) => Value::Null,
        Err(e) => Value::Null,
    }
}

pub fn sql_nonnull_to_json<'r>(mut get_ref: impl FnMut() -> sqlx::any::AnyValueRef<'r>) -> Value {
    let raw_value = get_ref();
    match raw_value.type_info().name() {
        "REAL" | "FLOAT" | "NUMERIC" | "DECIMAL" | "FLOAT4" | "FLOAT8" | "DOUBLE" => {
            <f64 as Decode<sqlx::any::Any>>::decode(raw_value)
                .unwrap_or(f64::NAN)
                .into()
        }
        "INT8" | "BIGINT" | "INTEGER" => <i64 as Decode<sqlx::any::Any>>::decode(raw_value)
            .unwrap_or_default()
            .into(),
        "INT" | "INT4" => <i32 as Decode<sqlx::any::Any>>::decode(raw_value)
            .unwrap_or_default()
            .into(),
        "INT2" | "SMALLINT" => <i16 as Decode<sqlx::any::Any>>::decode(raw_value)
            .unwrap_or_default()
            .into(),
        "BOOL" | "BOOLEAN" => <bool as Decode<sqlx::any::Any>>::decode(raw_value)
            .unwrap_or_default()
            .into(),
        /* "DATE" => <chrono::NaiveDate as Decode<sqlx::any::Any>>::decode(raw_value)
            .as_ref()
            .map_or_else(std::string::ToString::to_string, ToString::to_string)
            .into(),
        "TIME" => <chrono::NaiveTime as Decode<sqlx::any::Any>>::decode(raw_value)
            .as_ref()
            .map_or_else(ToString::to_string, ToString::to_string)
            .into(),
        "DATETIME" | "DATETIME2" | "DATETIMEOFFSET" | "TIMESTAMP" | "TIMESTAMPTZ" => {
            let mut date_time = <DateTime<Utc> as Decode<sqlx::any::Any>>::decode(get_ref());
            if date_time.is_err() {
                date_time = <chrono::NaiveDateTime as Decode<sqlx::any::Any>>::decode(raw_value)
                    .map(|d| d.and_utc());
            }
            Value::String(
                date_time
                    .as_ref()
                    .map_or_else(ToString::to_string, DateTime::to_rfc3339),
            )
        }
        "JSON" | "JSON[]" | "JSONB" | "JSONB[]" => {
            <Value as Decode<sqlx::any::Any>>::decode(raw_value).unwrap_or_default()
        }*/
        // Deserialize as a string by default
        _ => <String as Decode<sqlx::any::Any>>::decode(raw_value)
            .unwrap_or_default()
            .into(),
    }
}

/// Takes the first column of a row and converts it to a string.
pub fn row_to_string(row: &AnyRow) -> Option<String> {
    let col = row.columns().first()?;
    match sql_to_json(row, col) {
        serde_json::Value::String(s) => Some(s),
        serde_json::Value::Null => None,
        other => Some(other.to_string()),
    }
}
