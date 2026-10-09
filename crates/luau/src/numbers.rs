//! Integers at the boundary (#442 L6). Every value that leaves Luau passes `walk` before it becomes a
//! Rust value: a hook's return and every argument a host function reads. The rules count in `i32`,
//! while a Luau number is an `f64`, so a number crosses only when it is integral, finite and inside
//! `i32`; anything else is refused, naming the value. Tables
//! cross as JSON: a list (keys exactly `1..=n`) as an array in index order, a record (string keys
//! only) as an object with its keys sorted, so nothing that crosses depends on Luau's hash order.
//!
//! `J.div` and `J.rem` (`div`, `rem`) are the only division a script has: they truncate toward zero
//! as Rust's `/` and `%` do, where Luau's own `//` and `%` floor.

use std::fmt;

use mlua::{Error, Table, Value};
use serde::de::DeserializeOwned;
use serde_json::{Map, Value as Json};

use crate::config::LUAU_VALUE_DEPTH;
use crate::{LuauError, Site};

/// Why a value could not cross: a number that is not an `i32`, or a value of another kind no Rust
/// value stands for.
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

/// A Luau number as an `i32`, or the number's text when it is not one: a fraction (`"1.5"`), `NaN`,
/// an infinity (`"inf"`) or a value outside `i32` (`"2147483648"`). mlua hands an integral number
/// over as `Value::Integer`, anything else as `Value::Number`.
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

/// The walk: a value from Luau as JSON, or why it cannot cross.
fn walk(value: &Value, depth: usize) -> Result<Json, Refusal> {
    if depth > LUAU_VALUE_DEPTH {
        return Err(Refusal::Value(format!(
            "a value nested deeper than {LUAU_VALUE_DEPTH} tables cannot cross into Rust (L6)"
        )));
    }
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
        Value::Table(table) => walk_table(table, depth),
        other => Err(Refusal::Value(format!(
            "a value of type {} cannot cross into Rust (L6)",
            other.type_name()
        ))),
    }
}

/// A table as a list, a record or, empty, `{}`.
fn walk_table(table: &Table, depth: usize) -> Result<Json, Refusal> {
    let refused = |error: Error| Refusal::Value(error.to_string());
    let mut keys = Vec::new();
    for pair in table.pairs::<Value, Value>() {
        let (key, _) = pair.map_err(refused)?;
        keys.push(key);
    }
    if keys.is_empty() {
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
            items.push(walk(&item, depth + 1)?);
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
        record.insert(name, walk(&item, depth + 1)?);
    }
    Ok(Json::Object(record))
}

/// A hook's value as JSON, or the error that names its hook (and, for a number, the number).
pub fn to_json(value: &Value, site: Site) -> Result<Json, LuauError> {
    walk(value, 0).map_err(|refusal| match refusal {
        Refusal::Number(value) => LuauError::Number { site, value },
        Refusal::Value(message) => LuauError::Hook { site, message },
    })
}

/// A host function's argument, walked and then read as `T`. Every reader and `J` function reads its
/// arguments through this, so they cross by the same rules as a hook's return.
pub fn arg<T: DeserializeOwned>(value: Value) -> mlua::Result<T> {
    let json = walk(&value, 0).map_err(Error::runtime)?;
    serde_json::from_value(json).map_err(Error::runtime)
}

/// `J.div`: Rust's `/`, truncating toward zero. `None` where Rust's would panic: a zero divisor, or
/// `i32::MIN / -1`.
pub fn div(a: i32, b: i32) -> Option<i32> {
    a.checked_div(b)
}

/// `J.rem`: Rust's `%`, whose sign follows the dividend. `None` where Rust's would panic.
pub fn rem(a: i32, b: i32) -> Option<i32> {
    a.checked_rem(b)
}
