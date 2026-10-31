//! Integers at boundary (#442 L6). Every value leaving Luau passes `walk` to become Rust value.
//! `J.div` and `J.rem` truncate toward zero as Rust does.

use std::fmt;

use mlua::{Error, Table, Value};
use serde::de::DeserializeOwned;
use serde_json::{Map, Value as Json};

use crate::config::{LUAU_VALUE_DEPTH, LUAU_VALUE_NODES};
use crate::{LuauError, Site};

/// Why a value could not cross: non-integer or unrepresented value.
#[derive(Clone, Debug, PartialEq)]
pub enum Refusal {
    Number(String),
    Value(String),
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Refusal::Number(value) => write!(f, "{value} is not an integer in i32's range (L6)"),
            Refusal::Value(message) => f.write_str(message),
        }
    }
}

/// Luau number as `i32` or failure text.
pub fn number_to_i32(value: &Value) -> Result<i32, String> {
    match value {
        Value::Integer(n) => i32::try_from(*n).map_err(|_| n.to_string()),
        Value::Number(n) => {
            let whole = n.is_finite() && n.fract() == 0.0;
            if whole && *n >= f64::from(i32::MIN) && *n <= f64::from(i32::MAX) {
                Ok(*n as i32)
            } else {
                Err(n.to_string())
            }
        }
        other => Err(format!("a value of type {}", other.type_name())),
    }
}

/// Walks Luau value to JSON, enforcing `LUAU_VALUE_DEPTH` and `LUAU_VALUE_NODES`.
fn walk(value: &Value, depth: usize, nodes: &mut usize) -> Result<Json, Refusal> {
    if depth > LUAU_VALUE_DEPTH {
        return Err(Refusal::Value(format!(
            "a value nested deeper than {LUAU_VALUE_DEPTH} tables cannot cross into Rust (L6)"
        )));
    }
    count(nodes)?;
    match value {
        Value::Nil => Ok(Json::Null),
        Value::Boolean(b) => Ok(Json::Bool(*b)),
        Value::Integer(_) | Value::Number(_) => number_to_i32(value).map(Json::from).map_err(Refusal::Number),
        Value::String(s) => match s.to_str() {
            Ok(text) => Ok(Json::String(text.to_string())),
            Err(_) => Err(Refusal::Value(
                "a string that is not UTF-8 cannot cross into Rust (L6)".into(),
            )),
        },
        Value::Table(table) => walk_table(table, depth, nodes),
        other => Err(Refusal::Value(format!(
            "a value of type {} cannot cross into Rust (L6)",
            other.type_name()
        ))),
    }
}

/// One more value or key walked, or refusal if exceeding limit.
fn count(nodes: &mut usize) -> Result<(), Refusal> {
    *nodes += 1;
    if *nodes > LUAU_VALUE_NODES {
        return Err(Refusal::Value(format!(
            "a value of more than {LUAU_VALUE_NODES} values and keys cannot cross into Rust (L6)"
        )));
    }
    Ok(())
}

/// Walks table as list or record. Empty tables without metatables walk as records (#464 gap 1).
fn walk_table(table: &Table, depth: usize, nodes: &mut usize) -> Result<Json, Refusal> {
    let refused = |error: Error| Refusal::Value(error.to_string());
    let mut keys = Vec::new();
    for pair in table.pairs::<Value, Value>() {
        let (key, _) = pair.map_err(refused)?;
        count(nodes)?;
        keys.push(key);
    }
    if keys.is_empty() {
        if table.has_metatable() {
            return Ok(Json::Array(Vec::new()));
        }
        return Ok(Json::Object(Map::new()));
    }
    let len = table.raw_len();
    let is_list = keys.len() == len
        && keys
            .iter()
            .all(|key| matches!(key, Value::Integer(i) if usize::try_from(*i).is_ok_and(|i| (1..=len).contains(&i))));
    if is_list {
        let mut items = Vec::with_capacity(len);
        for index in 1..=len {
            let item: Value = table.raw_get(index).map_err(refused)?;
            items.push(walk(&item, depth + 1, nodes)?);
        }
        return Ok(Json::Array(items));
    }
    let mut names = Vec::with_capacity(keys.len());
    for key in &keys {
        let Value::String(name) = key else {
            return Err(Refusal::Value(
                "a table whose keys are neither 1..n nor all strings cannot cross into Rust (L6)".into(),
            ));
        };
        match name.to_str() {
            Ok(name) => names.push(name.to_string()),
            Err(_) => {
                return Err(Refusal::Value(
                    "a key that is not UTF-8 cannot cross into Rust (L6)".into(),
                ));
            }
        }
    }
    names.sort();
    let mut record = Map::new();
    for name in names {
        let item: Value = table.raw_get(name.as_str()).map_err(refused)?;
        record.insert(name, walk(&item, depth + 1, nodes)?);
    }
    Ok(Json::Object(record))
}

/// A hook's value as JSON, or the error that names its hook (and, for a number, the number).
pub fn to_json(value: &Value, site: Site) -> Result<Json, LuauError> {
    walk(value, 0, &mut 0).map_err(|refusal| match refusal {
        Refusal::Number(value) => LuauError::Number { site, value },
        Refusal::Value(message) => LuauError::Hook { site, message },
    })
}

/// A Luau number as an `i32`, or the error that names its hook.
pub fn to_i32(value: &Value, site: Site) -> Result<i32, LuauError> {
    number_to_i32(value).map_err(|value| LuauError::Number { site, value })
}

/// Host function argument walked and deserialized as `T`.
pub fn arg<T: DeserializeOwned>(value: Value) -> mlua::Result<T> {
    let json = walk(&value, 0, &mut 0).map_err(Error::runtime)?;
    match serde_json::from_value(json.clone()) {
        Ok(val) => Ok(val),
        // Retry with empty records coerced to empty list (#464 gap 1).
        Err(first) => serde_json::from_value(coerce_empty_objects(&json)).map_err(|_| Error::runtime(first)),
    }
}

/// Coerces empty records to empty lists recursively (#464 gap 1).
pub fn coerce_empty_objects(value: &Json) -> Json {
    match value {
        Json::Object(map) if map.is_empty() => Json::Array(Vec::new()),
        Json::Object(map) => {
            let mut coerced = Map::new();
            for (key, item) in map {
                coerced.insert(key.clone(), coerce_empty_objects(item));
            }
            Json::Object(coerced)
        }
        Json::Array(items) => Json::Array(items.iter().map(coerce_empty_objects).collect()),
        other => other.clone(),
    }
}

/// `J.div`: Rust's `/`, truncating toward zero. `None` where Rust's would panic.
pub fn div(a: i32, b: i32) -> Option<i32> {
    a.checked_div(b)
}

/// `J.rem`: Rust's `%`, whose sign follows the dividend. `None` where Rust's would panic.
pub fn rem(a: i32, b: i32) -> Option<i32> {
    a.checked_rem(b)
}
