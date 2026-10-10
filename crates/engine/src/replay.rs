//! Replay: (seed, decks, action log) rebuilds a match exactly, and a state hash makes two folds
//! comparable (SPEC §9.2, §9.3). A practice game adds its handicaps to that tuple (§9.9, R180, R187),
//! a game with a dealt deck the seats that were dealt one (R433), and a match its seats' last boards
//! (R417) and its Glitch boards (R678): they are setup, not actions, so the fold hands them to
//! `create_game` exactly as the live game did.
//!
//! The hash's text is fixed (`docs/v0.3.0/SURFACE.md` §5.2; §4.4.1 sort stability, §4.4.7 absent
//! options; §6.1 `fold`'s signature): practice saves on players' devices store the hash, the hotseat
//! fixture pins `"a798906b"`, and the golden traces (§13) and the server's migration checksum use the
//! same `canonical` and `fnv1a32_utf16`.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::config::{Handicap, REPLAY_CHECKPOINT_EVERY, REPLAY_PAGE_STEPS};
use crate::state::{CreateGameOptions, GameState, LastBoardInput, create_game};
use crate::view_for::view_for;
use crate::wire::{
    Action, CardDefs, PerPlayerOpt, PlayerId, ReplayCheckpoints, ReplayOpen, ReplayPage, ReplayRecord,
    ReplayRefusal, ReplayStep,
};

/// Canonical JSON: keys sorted, so two equal states always produce the same text. A string, number,
/// bool or null as `JSON.stringify` writes it; an array as `[a,b]`; an object as `{"key":value}` with
/// its keys sorted ascending by UTF-16 code units. An absent `Option` is no key at all.
pub fn canonical(value: &Value) -> String {
    let mut out = String::new();
    write_canonical(value, &mut out);
    out
}

fn write_canonical(value: &Value, out: &mut String) {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(flag) => out.push_str(if *flag { "true" } else { "false" }),
        Value::Number(number) => out.push_str(&js_number(number)),
        Value::String(text) => out.push_str(&json_string(text)),
        Value::Array(items) => {
            out.push('[');
            for (at, item) in items.iter().enumerate() {
                if at > 0 {
                    out.push(',');
                }
                write_canonical(item, out);
            }
            out.push(']');
        }
        Value::Object(map) => {
            let mut entries: Vec<(&String, &Value)> = map.iter().collect();
            // UTF-16 code unit order, a stable sort; keys are unique.
            entries.sort_by(|(a, _), (b, _)| a.encode_utf16().cmp(b.encode_utf16()));
            out.push('{');
            for (at, (key, item)) in entries.into_iter().enumerate() {
                if at > 0 {
                    out.push(',');
                }
                out.push_str(&json_string(key));
                out.push(':');
                write_canonical(item, out);
            }
            out.push('}');
        }
    }
}

/// `JSON.stringify(text)`: serde_json writes the same escapes (`\" \\ \b \f \n \r \t`, `\u00xx` in
/// lower-case hex for the other control characters, everything else raw).
fn json_string(text: &str) -> String {
    serde_json::to_string(text).unwrap_or_else(|_| String::from("\"\""))
}

/// `JSON.stringify(n)` for a JSON number: an integer as its digits; a float with no fraction as an
/// integer (JS has one number type, so `2.0` prints `2`); any other float as serde_json writes it,
/// which is the shortest round-trip form, as JS's is. The engine's state holds integers only.
fn js_number(number: &serde_json::Number) -> String {
    if number.is_i64() || number.is_u64() {
        return number.to_string();
    }
    match number.as_f64() {
        Some(float) if float.is_finite() && float.fract() == 0.0 && float.abs() < 1e21 => {
            format!("{float:.0}")
        }
        Some(float) if !float.is_finite() => String::from("null"),
        _ => number.to_string(),
    }
}

/// FNV-1a 32 over the UTF-16 code units of `text`, printed as 8 lower-case hex digits. UTF-8 bytes
/// would differ: state strings carry `× − – ♾ ³ ²`.
pub fn fnv1a32_utf16(text: &str) -> String {
    let mut hash: u32 = 0x811c_9dc5;
    let mut step = |unit: u16| {
        hash ^= u32::from(unit);
        hash = hash.wrapping_mul(0x0100_0193);
    };
    // An ASCII byte is its own UTF-16 code unit; any other character is encoded as it comes.
    let mut rest = text;
    while !rest.is_empty() {
        let ascii = rest
            .bytes()
            .position(|byte| !byte.is_ascii())
            .unwrap_or(rest.len());
        for &byte in &rest.as_bytes()[..ascii] {
            step(u16::from(byte));
        }
        rest = &rest[ascii..];
        let Some(ch) = rest.chars().next() else {
            break;
        };
        for &unit in ch.encode_utf16(&mut [0u16; 2]).iter() {
            step(unit);
        }
        rest = &rest[ch.len_utf8()..];
    }
    format!("{hash:08x}")
}

/// FNV-1a over the canonical state, minus the nonce log, which is bookkeeping, and the opening a Glitch
/// reset deals again (R676), which is a copy of the fold's own input, so no hash moved when it came.
/// The text is written straight from `Serialize` (`direct::canonical_text`), byte for byte what
/// `canonical(&to_value(state))` writes; a shape that writer leaves alone takes the `Value` path
/// below, so the hash never depends on which path ran.
pub fn hash_state(state: &GameState) -> String {
    if let Some(text) = direct::canonical_text(state, &["applied", "opening"]) {
        return fnv1a32_utf16(&text);
    }
    let mut value = match serde_json::to_value(state) {
        Ok(value) => value,
        Err(error) => panic!("hash_state: a GameState did not serialise: {error}"),
    };
    if let Value::Object(map) = &mut value {
        map.remove("applied");
        map.remove("opening");
    }
    fnv1a32_utf16(&canonical(&value))
}

/// `canonical(&serde_json::to_value(value))`, written from `Serialize` without the `Value` tree. Every
/// shape is written as `to_value` would build it: an object's keys in UTF-16 order (a later duplicate
/// replacing an earlier, as a `Value` map's insert does), `None` and `()` as `null`, a unit variant as
/// its name, a newtype variant as `{"variant":value}`, an `f64` through `Value::from` and `js_number`.
/// What it cannot write (128-bit integers, bytes, a map key that is not a string, char, bool or
/// integer) is an error, and the caller takes the `Value` path instead.
///
/// Entries go out as they come and are put in key order when the object closes. A struct's keys are
/// its `&'static str` field names, so their sorted order is worked out once per set and remembered
/// (`Out::orders`, keyed by the names' addresses and lengths: the same pair is the same text).
mod direct {
    use std::borrow::Cow;
    use std::cmp::Ordering;
    use std::fmt;

    use indexmap::IndexMap;
    use serde::ser::{self, Serialize};
    use serde_json::Value;

    /// A shape left to the `Value` path.
    #[derive(Debug)]
    pub struct Unsupported;

    impl fmt::Display for Unsupported {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("left to the Value path")
        }
    }

    impl std::error::Error for Unsupported {}

    impl ser::Error for Unsupported {
        fn custom<T: fmt::Display>(_msg: T) -> Self {
            Unsupported
        }
    }

    type Result<T = ()> = std::result::Result<T, Unsupported>;

    /// A map key, borrowed when it is a struct's field name or a variant's.
    type Key = Cow<'static, str>;

    /// How a set of entries goes out: `None` as written, else these indexes, in key order, with every
    /// replaced duplicate left out.
    type Order = Option<Vec<u32>>;

    /// The text being written, and the orders worked out for struct key sets so far.
    struct Out {
        bytes: Vec<u8>,
        orders: IndexMap<Vec<usize>, Order, crate::catalog::IdHash>,
        /// Scratch for a signature being looked up.
        signature: Vec<usize>,
    }

    /// The canonical text of `value`, leaving out the top-level object's keys named in `skip`; `None`
    /// when `value` holds a shape this writer leaves to the `Value` path.
    pub fn canonical_text<T: Serialize + ?Sized>(value: &T, skip: &'static [&'static str]) -> Option<String> {
        let mut out = Out {
            bytes: Vec::with_capacity(1 << 16),
            orders: IndexMap::default(),
            signature: Vec::new(),
        };
        value.serialize(Writer { out: &mut out, skip }).ok()?;
        String::from_utf8(out.bytes).ok()
    }

    /// `JSON.stringify(text)`, as `json_string` writes it (serde_json's own escaping).
    fn push_str(out: &mut Vec<u8>, text: &str) -> Result {
        serde_json::to_writer(&mut *out, text).map_err(|_| Unsupported)
    }

    fn push_u64(out: &mut Vec<u8>, mut value: u64) {
        let mut digits = [0u8; 20];
        let mut at = digits.len();
        loop {
            at -= 1;
            digits[at] = b'0' + (value % 10) as u8;
            value /= 10;
            if value == 0 {
                break;
            }
        }
        out.extend_from_slice(&digits[at..]);
    }

    fn push_i64(out: &mut Vec<u8>, value: i64) {
        if value < 0 {
            out.push(b'-');
        }
        push_u64(out, value.unsigned_abs());
    }

    /// A float as `to_value` stores it (`Value::from`) and `canonical` writes it.
    fn push_float(out: &mut Vec<u8>, value: Value) {
        let mut text = String::new();
        super::write_canonical(&value, &mut text);
        out.extend_from_slice(text.as_bytes());
    }

    struct Writer<'a> {
        out: &'a mut Out,
        /// Keys the object being written leaves out (only ever the top level's).
        skip: &'static [&'static str],
    }

    impl<'a> Writer<'a> {
        fn bytes(&mut self) -> &mut Vec<u8> {
            &mut self.out.bytes
        }

        fn object(self, close: &'static [u8]) -> Object<'a> {
            self.out.bytes.push(b'{');
            let start = self.out.bytes.len();
            Object {
                out: self.out,
                skip: self.skip,
                start,
                entries: Vec::new(),
                key: None,
                close,
            }
        }

        fn list(self, close: &'static [u8]) -> List<'a> {
            self.out.bytes.push(b'[');
            List {
                out: self.out,
                first: true,
                close,
            }
        }

        /// `{"variant":` before a variant's content; the content's close adds the outer `}`.
        fn open_variant(&mut self, variant: &str) -> Result {
            self.bytes().push(b'{');
            push_str(self.bytes(), variant)?;
            self.bytes().push(b':');
            Ok(())
        }
    }

    impl<'a> ser::Serializer for Writer<'a> {
        type Ok = ();
        type Error = Unsupported;
        type SerializeSeq = List<'a>;
        type SerializeTuple = List<'a>;
        type SerializeTupleStruct = List<'a>;
        type SerializeTupleVariant = List<'a>;
        type SerializeMap = Object<'a>;
        type SerializeStruct = Object<'a>;
        type SerializeStructVariant = Object<'a>;

        fn serialize_bool(mut self, value: bool) -> Result {
            self.bytes()
                .extend_from_slice(if value { b"true" } else { b"false" });
            Ok(())
        }
        fn serialize_i8(mut self, value: i8) -> Result {
            push_i64(self.bytes(), i64::from(value));
            Ok(())
        }
        fn serialize_i16(mut self, value: i16) -> Result {
            push_i64(self.bytes(), i64::from(value));
            Ok(())
        }
        fn serialize_i32(mut self, value: i32) -> Result {
            push_i64(self.bytes(), i64::from(value));
            Ok(())
        }
        fn serialize_i64(mut self, value: i64) -> Result {
            push_i64(self.bytes(), value);
            Ok(())
        }
        fn serialize_u8(mut self, value: u8) -> Result {
            push_u64(self.bytes(), u64::from(value));
            Ok(())
        }
        fn serialize_u16(mut self, value: u16) -> Result {
            push_u64(self.bytes(), u64::from(value));
            Ok(())
        }
        fn serialize_u32(mut self, value: u32) -> Result {
            push_u64(self.bytes(), u64::from(value));
            Ok(())
        }
        fn serialize_u64(mut self, value: u64) -> Result {
            push_u64(self.bytes(), value);
            Ok(())
        }
        fn serialize_i128(self, _value: i128) -> Result {
            Err(Unsupported)
        }
        fn serialize_u128(self, _value: u128) -> Result {
            Err(Unsupported)
        }
        fn serialize_f32(mut self, value: f32) -> Result {
            push_float(self.bytes(), Value::from(value));
            Ok(())
        }
        fn serialize_f64(mut self, value: f64) -> Result {
            push_float(self.bytes(), Value::from(value));
            Ok(())
        }
        fn serialize_char(mut self, value: char) -> Result {
            push_str(self.bytes(), value.encode_utf8(&mut [0u8; 4]))
        }
        fn serialize_str(mut self, value: &str) -> Result {
            push_str(self.bytes(), value)
        }
        fn serialize_bytes(self, _value: &[u8]) -> Result {
            Err(Unsupported)
        }
        fn serialize_none(mut self) -> Result {
            self.bytes().extend_from_slice(b"null");
            Ok(())
        }
        fn serialize_some<T: Serialize + ?Sized>(self, value: &T) -> Result {
            value.serialize(self)
        }
        fn serialize_unit(mut self) -> Result {
            self.bytes().extend_from_slice(b"null");
            Ok(())
        }
        fn serialize_unit_struct(mut self, _name: &'static str) -> Result {
            self.bytes().extend_from_slice(b"null");
            Ok(())
        }
        fn serialize_unit_variant(
            mut self,
            _name: &'static str,
            _index: u32,
            variant: &'static str,
        ) -> Result {
            push_str(self.bytes(), variant)
        }
        fn serialize_newtype_struct<T: Serialize + ?Sized>(self, _name: &'static str, value: &T) -> Result {
            value.serialize(self)
        }
        fn serialize_newtype_variant<T: Serialize + ?Sized>(
            mut self,
            _name: &'static str,
            _index: u32,
            variant: &'static str,
            value: &T,
        ) -> Result {
            self.open_variant(variant)?;
            value.serialize(Writer {
                out: &mut *self.out,
                skip: &[],
            })?;
            self.bytes().push(b'}');
            Ok(())
        }
        fn serialize_seq(self, _len: Option<usize>) -> Result<List<'a>> {
            Ok(self.list(b"]"))
        }
        fn serialize_tuple(self, _len: usize) -> Result<List<'a>> {
            Ok(self.list(b"]"))
        }
        fn serialize_tuple_struct(self, _name: &'static str, _len: usize) -> Result<List<'a>> {
            Ok(self.list(b"]"))
        }
        fn serialize_tuple_variant(
            mut self,
            _name: &'static str,
            _index: u32,
            variant: &'static str,
            _len: usize,
        ) -> Result<List<'a>> {
            self.open_variant(variant)?;
            Ok(self.list(b"]}"))
        }
        fn serialize_map(self, _len: Option<usize>) -> Result<Object<'a>> {
            Ok(self.object(b"}"))
        }
        fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<Object<'a>> {
            Ok(self.object(b"}"))
        }
        fn serialize_struct_variant(
            mut self,
            _name: &'static str,
            _index: u32,
            variant: &'static str,
            _len: usize,
        ) -> Result<Object<'a>> {
            self.open_variant(variant)?;
            Ok(Writer { skip: &[], ..self }.object(b"}}"))
        }
    }

    struct List<'a> {
        out: &'a mut Out,
        first: bool,
        close: &'static [u8],
    }

    impl List<'_> {
        fn item<T: Serialize + ?Sized>(&mut self, value: &T) -> Result {
            if !self.first {
                self.out.bytes.push(b',');
            }
            self.first = false;
            value.serialize(Writer {
                out: &mut *self.out,
                skip: &[],
            })
        }
        fn close(self) -> Result {
            self.out.bytes.extend_from_slice(self.close);
            Ok(())
        }
    }

    impl ser::SerializeSeq for List<'_> {
        type Ok = ();
        type Error = Unsupported;
        fn serialize_element<T: Serialize + ?Sized>(&mut self, value: &T) -> Result {
            self.item(value)
        }
        fn end(self) -> Result {
            self.close()
        }
    }

    impl ser::SerializeTuple for List<'_> {
        type Ok = ();
        type Error = Unsupported;
        fn serialize_element<T: Serialize + ?Sized>(&mut self, value: &T) -> Result {
            self.item(value)
        }
        fn end(self) -> Result {
            self.close()
        }
    }

    impl ser::SerializeTupleStruct for List<'_> {
        type Ok = ();
        type Error = Unsupported;
        fn serialize_field<T: Serialize + ?Sized>(&mut self, value: &T) -> Result {
            self.item(value)
        }
        fn end(self) -> Result {
            self.close()
        }
    }

    impl ser::SerializeTupleVariant for List<'_> {
        type Ok = ();
        type Error = Unsupported;
        fn serialize_field<T: Serialize + ?Sized>(&mut self, value: &T) -> Result {
            self.item(value)
        }
        fn end(self) -> Result {
            self.close()
        }
    }

    /// One entry of an object: its key and the span of its `"key":value` text in the output.
    struct Entry {
        key: Key,
        /// No byte of the key is 0xEE or above, so it holds no character from U+E000 up, and its UTF-8
        /// byte order against another such key is its UTF-16 order.
        plain: bool,
        from: usize,
        to: usize,
    }

    impl Entry {
        /// Key order by UTF-16 code units. UTF-8 byte order is code point order, which differs from
        /// UTF-16's only between a character from U+E000 to U+FFFF and one past U+FFFF.
        fn cmp_key(&self, other: &Entry) -> Ordering {
            if self.plain && other.plain {
                self.key.as_bytes().cmp(other.key.as_bytes())
            } else {
                self.key.encode_utf16().cmp(other.key.encode_utf16())
            }
        }
    }

    /// The order `entries` go out in (`Order`): sorted by key, stably, so of equal keys the one
    /// written last is last, and that is the one kept, as a map keeps it.
    fn order_of(entries: &[Entry]) -> Order {
        if entries
            .windows(2)
            .all(|pair| pair[0].cmp_key(&pair[1]) == Ordering::Less)
        {
            return None;
        }
        let mut order: Vec<u32> = (0..entries.len() as u32).collect();
        order.sort_by(|&a, &b| entries[a as usize].cmp_key(&entries[b as usize]));
        let mut kept: Vec<u32> = Vec::with_capacity(order.len());
        for (at, &index) in order.iter().enumerate() {
            let replaced = order.get(at + 1).is_some_and(|&next| {
                entries[next as usize].cmp_key(&entries[index as usize]) == Ordering::Equal
            });
            if !replaced {
                kept.push(index);
            }
        }
        Some(kept)
    }

    /// An object being written: its entries go out in the order they come, and `close` puts them in
    /// key order (dropping a replaced duplicate) when they are not in it already.
    struct Object<'a> {
        out: &'a mut Out,
        skip: &'static [&'static str],
        /// The first byte after `{`.
        start: usize,
        entries: Vec<Entry>,
        /// A map's key, between `serialize_key` and `serialize_value`.
        key: Option<Key>,
        close: &'static [u8],
    }

    impl Object<'_> {
        fn entry<T: Serialize + ?Sized>(&mut self, key: Key, value: &T) -> Result {
            if self.skip.contains(&key.as_ref()) {
                return Ok(());
            }
            if !self.entries.is_empty() {
                self.out.bytes.push(b',');
            }
            let from = self.out.bytes.len();
            push_str(&mut self.out.bytes, &key)?;
            self.out.bytes.push(b':');
            value.serialize(Writer {
                out: &mut *self.out,
                skip: &[],
            })?;
            let to = self.out.bytes.len();
            let plain = key.bytes().all(|byte| byte < 0xEE);
            self.entries.push(Entry { key, plain, from, to });
            Ok(())
        }

        /// The entries' order: a struct's key set's remembered one (`Out::orders`), else sorted now.
        fn order(&mut self) -> Order {
            let out = &mut *self.out;
            out.signature.clear();
            for entry in &self.entries {
                let Cow::Borrowed(name) = entry.key else {
                    return order_of(&self.entries);
                };
                out.signature.push(name.as_ptr() as usize);
                out.signature.push(name.len());
            }
            if let Some(order) = out.orders.get(out.signature.as_slice()) {
                return order.clone();
            }
            let order = order_of(&self.entries);
            out.orders.insert(out.signature.clone(), order.clone());
            order
        }

        fn close(mut self) -> Result {
            if let Some(order) = self.order() {
                let bytes = &mut self.out.bytes;
                let region = bytes[self.start..].to_vec();
                bytes.truncate(self.start);
                for (at, &index) in order.iter().enumerate() {
                    if at > 0 {
                        bytes.push(b',');
                    }
                    let entry = &self.entries[index as usize];
                    bytes.extend_from_slice(&region[entry.from - self.start..entry.to - self.start]);
                }
            }
            self.out.bytes.extend_from_slice(self.close);
            Ok(())
        }
    }

    impl ser::SerializeMap for Object<'_> {
        type Ok = ();
        type Error = Unsupported;
        fn serialize_key<T: Serialize + ?Sized>(&mut self, key: &T) -> Result {
            self.key = Some(key.serialize(KeyWriter)?);
            Ok(())
        }
        fn serialize_value<T: Serialize + ?Sized>(&mut self, value: &T) -> Result {
            let key = self.key.take().ok_or(Unsupported)?;
            self.entry(key, value)
        }
        fn end(self) -> Result {
            self.close()
        }
    }

    impl ser::SerializeStruct for Object<'_> {
        type Ok = ();
        type Error = Unsupported;
        fn serialize_field<T: Serialize + ?Sized>(&mut self, key: &'static str, value: &T) -> Result {
            self.entry(Cow::Borrowed(key), value)
        }
        fn end(self) -> Result {
            self.close()
        }
    }

    impl ser::SerializeStructVariant for Object<'_> {
        type Ok = ();
        type Error = Unsupported;
        fn serialize_field<T: Serialize + ?Sized>(&mut self, key: &'static str, value: &T) -> Result {
            self.entry(Cow::Borrowed(key), value)
        }
        fn end(self) -> Result {
            self.close()
        }
    }

    /// A map key as `to_value` takes it: a string, a char, a bool or an integer as its text, a unit
    /// variant as its name, a newtype as what it wraps. Anything else is left to the `Value` path. A
    /// map's keys are owned (`Cow::Owned`), so its order is never taken for a struct's.
    struct KeyWriter;

    type NoKey = ser::Impossible<Key, Unsupported>;

    fn owned(text: String) -> Result<Key> {
        Ok(Cow::Owned(text))
    }

    impl ser::Serializer for KeyWriter {
        type Ok = Key;
        type Error = Unsupported;
        type SerializeSeq = NoKey;
        type SerializeTuple = NoKey;
        type SerializeTupleStruct = NoKey;
        type SerializeTupleVariant = NoKey;
        type SerializeMap = NoKey;
        type SerializeStruct = NoKey;
        type SerializeStructVariant = NoKey;

        fn serialize_bool(self, value: bool) -> Result<Key> {
            owned(value.to_string())
        }
        fn serialize_i8(self, value: i8) -> Result<Key> {
            owned(value.to_string())
        }
        fn serialize_i16(self, value: i16) -> Result<Key> {
            owned(value.to_string())
        }
        fn serialize_i32(self, value: i32) -> Result<Key> {
            owned(value.to_string())
        }
        fn serialize_i64(self, value: i64) -> Result<Key> {
            owned(value.to_string())
        }
        fn serialize_u8(self, value: u8) -> Result<Key> {
            owned(value.to_string())
        }
        fn serialize_u16(self, value: u16) -> Result<Key> {
            owned(value.to_string())
        }
        fn serialize_u32(self, value: u32) -> Result<Key> {
            owned(value.to_string())
        }
        fn serialize_u64(self, value: u64) -> Result<Key> {
            owned(value.to_string())
        }
        fn serialize_f32(self, _value: f32) -> Result<Key> {
            Err(Unsupported)
        }
        fn serialize_f64(self, _value: f64) -> Result<Key> {
            Err(Unsupported)
        }
        fn serialize_char(self, value: char) -> Result<Key> {
            owned(value.to_string())
        }
        fn serialize_str(self, value: &str) -> Result<Key> {
            owned(value.to_string())
        }
        fn serialize_bytes(self, _value: &[u8]) -> Result<Key> {
            Err(Unsupported)
        }
        fn serialize_none(self) -> Result<Key> {
            Err(Unsupported)
        }
        fn serialize_some<T: Serialize + ?Sized>(self, _value: &T) -> Result<Key> {
            Err(Unsupported)
        }
        fn serialize_unit(self) -> Result<Key> {
            Err(Unsupported)
        }
        fn serialize_unit_struct(self, _name: &'static str) -> Result<Key> {
            Err(Unsupported)
        }
        fn serialize_unit_variant(
            self,
            _name: &'static str,
            _index: u32,
            variant: &'static str,
        ) -> Result<Key> {
            owned(variant.to_string())
        }
        fn serialize_newtype_struct<T: Serialize + ?Sized>(
            self,
            _name: &'static str,
            value: &T,
        ) -> Result<Key> {
            value.serialize(self)
        }
        fn serialize_newtype_variant<T: Serialize + ?Sized>(
            self,
            _name: &'static str,
            _index: u32,
            _variant: &'static str,
            _value: &T,
        ) -> Result<Key> {
            Err(Unsupported)
        }
        fn serialize_seq(self, _len: Option<usize>) -> Result<NoKey> {
            Err(Unsupported)
        }
        fn serialize_tuple(self, _len: usize) -> Result<NoKey> {
            Err(Unsupported)
        }
        fn serialize_tuple_struct(self, _name: &'static str, _len: usize) -> Result<NoKey> {
            Err(Unsupported)
        }
        fn serialize_tuple_variant(
            self,
            _name: &'static str,
            _index: u32,
            _variant: &'static str,
            _len: usize,
        ) -> Result<NoKey> {
            Err(Unsupported)
        }
        fn serialize_map(self, _len: Option<usize>) -> Result<NoKey> {
            Err(Unsupported)
        }
        fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<NoKey> {
            Err(Unsupported)
        }
        fn serialize_struct_variant(
            self,
            _name: &'static str,
            _index: u32,
            _variant: &'static str,
            _len: usize,
        ) -> Result<NoKey> {
            Err(Unsupported)
        }
    }
}

/// What a fold starts from.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ReplayInput {
    pub seed: String,
    pub decks: (Vec<String>, Vec<String>),
    pub log: Vec<Action>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub catalog: Option<CardDefs>,
    /// R180, R187: the same handicaps the live createGame had.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub handicaps: Option<PerPlayerOpt<Handicap>>,
    /// R433: the same dealt seats the live createGame had.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dealt: Option<Vec<PlayerId>>,
    /// R417: the same last boards the live createGame had.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_boards: Option<LastBoardInput>,
    /// R678: the same Glitch boards the live createGame had.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub glitch_boards: Option<LastBoardInput>,
}

pub type FoldArgs = ReplayInput;

/// One rejected action of a folded log: its nonce and the reducer's refusal, verbatim.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FoldError {
    pub nonce: String,
    pub error: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ReplayResult {
    pub state: GameState,
    pub errors: Vec<FoldError>,
}

pub type FoldResult = ReplayResult;

/// Fold a recorded log from scratch. Errors are collected, not thrown: a log may hold rejects. The
/// setup is not: `create_game` throws on a deck its seat's handicap does not allow, so a handicapped
/// game folded without its handicaps fails loudly instead of replaying a different game (R180).
pub fn fold(input: &ReplayInput) -> ReplayResult {
    let mut state = opening(input);
    let mut errors: Vec<FoldError> = Vec::new();

    for action in &input.log {
        let result = crate::reduce::reduce(&state, action);
        if let Some(error) = result.error {
            errors.push(FoldError {
                nonce: action.nonce.clone(),
                error,
            });
            continue;
        }
        state = result.state;
    }

    ReplayResult { state, errors }
}

/// The state after `begin_game`: `fold`'s start, and a replay's step 0 (R768).
fn opening(input: &ReplayInput) -> GameState {
    let start = create_game(&CreateGameOptions {
        seed: input.seed.clone(),
        decks: input.decks.clone(),
        catalog: input.catalog.clone(),
        handicaps: input.handicaps.clone(),
        dealt: input.dealt.clone(),
        last_boards: input.last_boards.clone(),
        glitch_boards: input.glitch_boards.clone(),
    });
    crate::reduce::begin_game(&start).state
}

/// R768: fold a finished game once and check it before any step is shown: another catalog version
/// than `build_version` is refused as `earlier_patch` before anything is folded, a log that does not
/// fold to `record.final_hash` as `rules_changed`. Otherwise the answer holds the step count (step 0
/// is the state after `begin_game`, step k the one after the k-th accepted action; a rejected one is
/// no step) and a state every `REPLAY_CHECKPOINT_EVERY` accepted actions. As with `fold`, a setup
/// `create_game` refuses panics; the WASM binding checks it first.
pub fn replay_open(input: &ReplayInput, record: &ReplayRecord, build_version: &str) -> ReplayOpen {
    if record.catalog_version != build_version {
        return ReplayOpen::Refused {
            reason: ReplayRefusal::EarlierPatch,
        };
    }
    let mut state = opening(input);
    let mut checkpoints = ReplayCheckpoints {
        accepted: Vec::new(),
        states: vec![state.clone()],
    };
    for action in &input.log {
        let result = crate::reduce::reduce(&state, action);
        if result.error.is_some() {
            continue;
        }
        state = result.state;
        checkpoints.accepted.push(action.clone());
        if checkpoints.accepted.len().is_multiple_of(REPLAY_CHECKPOINT_EVERY) {
            checkpoints.states.push(state.clone());
        }
    }
    if hash_state(&state) != record.final_hash {
        return ReplayOpen::Refused {
            reason: ReplayRefusal::RulesChanged,
        };
    }
    ReplayOpen::Ready {
        steps: checkpoints.accepted.len() + 1,
        checkpoints,
    }
}

/// R768: steps `[from, from + count)` of an opened replay as `seat` saw them, each `view_for` (never
/// the clock's) and its turn. `count` is capped at REPLAY_PAGE_STEPS and at the last step, and the
/// page ends early rather than make more than REPLAY_CHECKPOINT_EVERY `reduce` calls, so the next
/// page starts at `from + steps.len()`. `reduces` counts them (a pure crate keeps no counter).
pub fn replay_page(checkpoints: &ReplayCheckpoints, seat: PlayerId, from: usize, count: usize) -> ReplayPage {
    let total = checkpoints.accepted.len() + 1;
    let wanted = count.min(REPLAY_PAGE_STEPS).min(total.saturating_sub(from));
    let mut page = ReplayPage {
        from,
        steps: Vec::new(),
        reduces: 0,
    };
    let mut at = from - from % REPLAY_CHECKPOINT_EVERY;
    let Some(mut state) = checkpoints.states.get(at / REPLAY_CHECKPOINT_EVERY).cloned() else {
        return page;
    };
    while page.steps.len() < wanted {
        if at >= from {
            page.steps.push(ReplayStep {
                step: at,
                turn: state.turn,
                view: view_for(&state, seat),
            });
            if page.steps.len() == wanted {
                break;
            }
        }
        let next = at + 1;
        let saved = if next.is_multiple_of(REPLAY_CHECKPOINT_EVERY) {
            checkpoints.states.get(next / REPLAY_CHECKPOINT_EVERY)
        } else {
            None
        };
        if let Some(saved) = saved {
            state = saved.clone();
        } else {
            let Some(action) = checkpoints.accepted.get(at) else {
                break;
            };
            if page.reduces == REPLAY_CHECKPOINT_EVERY {
                break;
            }
            let result = crate::reduce::reduce(&state, action);
            page.reduces += 1;
            if result.error.is_some() {
                break;
            }
            state = result.state;
        }
        at = next;
    }
    page
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn canonical_sorts_keys_and_writes_json_stringify_text() {
        let value = json!({ "b": [1, "x\n", null, true], "a": { "z": 0, "y": "×" } });
        assert_eq!(
            canonical(&value),
            r#"{"a":{"y":"×","z":0},"b":[1,"x\n",null,true]}"#
        );
        assert_eq!(canonical(&json!(2.0)), "2");
        assert_eq!(canonical(&json!("\u{1}")), r#""\u0001""#);
    }

    /// `hash_state`'s direct writer writes exactly `canonical(&to_value(..))` for every shape serde
    /// derives: unsorted struct fields, flattened duplicates (the later one kept, as a `Value` map's
    /// insert keeps it), every enum representation, floats, escapes and keys outside the BMP.
    #[test]
    fn the_direct_writer_writes_what_canonical_writes() {
        use indexmap::IndexMap;

        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Inner {
            zeta: i32,
            alpha: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            gone: Option<i32>,
            mid: Vec<Shape>,
        }

        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        enum Shape {
            Unit,
            Newtype(i64),
            Tuple(u8, String),
            Struct { b: bool, a: f64 },
        }

        #[derive(Serialize)]
        #[serde(tag = "type", rename_all = "camelCase")]
        enum Tagged {
            First { x: i32, a: String },
            Second(Inner),
        }

        #[derive(Serialize)]
        struct Flat {
            zed: i32,
            #[serde(flatten)]
            rest: IndexMap<String, Value>,
            #[serde(rename = "dup")]
            later: i32,
        }

        #[derive(Serialize)]
        struct Top {
            seed: String,
            applied: Vec<i32>,
            inner: Inner,
            tagged: Vec<Tagged>,
            flat: Flat,
            map: IndexMap<String, Value>,
            floats: Vec<f64>,
            ints: (i8, u64, i64),
            unit: (),
            ch: char,
            // The same struct with a field present, absent and present again: each key set's order.
            more: Vec<Inner>,
        }

        let mut rest = IndexMap::new();
        rest.insert("dup".to_string(), json!("first"));
        rest.insert("b".to_string(), json!([1, 2.5, null]));
        let mut map = IndexMap::new();
        for key in ["z", "\u{ff61}", "\u{1f600}", "A", "é", "a\"b", "10", "9"] {
            map.insert(key.to_string(), json!({ "y": key, "x": [key] }));
        }
        let inner = || Inner {
            zeta: -3,
            alpha: None,
            gone: None,
            mid: vec![
                Shape::Unit,
                Shape::Newtype(-7),
                Shape::Tuple(255, "t\u{1}\n".to_string()),
                Shape::Struct { b: true, a: 2.0 },
            ],
        };
        let top = Top {
            seed: "s × − ♾ ³".to_string(),
            applied: vec![1, 2, 3],
            inner: inner(),
            tagged: vec![
                Tagged::First {
                    x: 1,
                    a: "q".to_string(),
                },
                Tagged::Second(inner()),
            ],
            flat: Flat {
                zed: 0,
                rest,
                later: 9,
            },
            map,
            floats: vec![
                0.0,
                -0.0,
                1.5,
                2.0,
                1e21,
                1e-7,
                f64::NAN,
                f64::INFINITY,
                123456789.0,
            ],
            ints: (i8::MIN, u64::MAX, i64::MIN),
            unit: (),
            ch: '\u{1f600}',
            more: vec![
                Inner {
                    gone: Some(1),
                    ..inner()
                },
                inner(),
                Inner {
                    gone: Some(2),
                    alpha: Some("a".to_string()),
                    ..inner()
                },
            ],
        };
        let value = serde_json::to_value(&top).expect("serialises");
        assert_eq!(
            direct::canonical_text(&top, &[]).as_deref(),
            Some(canonical(&value).as_str())
        );
        let mut without = value.clone();
        if let Value::Object(fields) = &mut without {
            fields.remove("applied");
            fields.remove("seed");
        }
        assert_eq!(
            direct::canonical_text(&top, &["applied", "seed"]).as_deref(),
            Some(canonical(&without).as_str())
        );
        // A `Value` itself, and a shape left to the `Value` path.
        assert_eq!(
            direct::canonical_text(&value, &[]).as_deref(),
            Some(canonical(&value).as_str())
        );
        assert_eq!(direct::canonical_text(&(1u128 << 70), &[]), None);
    }

    #[test]
    fn fnv_runs_over_utf16_code_units() {
        // FNV-1a 32 of the empty string is its offset basis.
        assert_eq!(fnv1a32_utf16(""), "811c9dc5");
        // "a" = 0x61: (0x811c9dc5 ^ 0x61) * 0x01000193 mod 2^32.
        assert_eq!(fnv1a32_utf16("a"), "e40c292c");
        // One UTF-16 unit for "×" (U+00D7), not its two UTF-8 bytes.
        let by_unit = {
            let mut hash: u32 = 0x811c_9dc5;
            hash ^= 0xd7;
            hash = hash.wrapping_mul(0x0100_0193);
            format!("{hash:08x}")
        };
        assert_eq!(fnv1a32_utf16("×"), by_unit);
        // ASCII runs, two-unit characters and the code units of a surrogate pair, against the plain loop.
        let mixed = "ab × − ♾ ³ 😀 \u{ff61} z\u{10ffff}";
        let mut hash: u32 = 0x811c_9dc5;
        for unit in mixed.encode_utf16() {
            hash ^= u32::from(unit);
            hash = hash.wrapping_mul(0x0100_0193);
        }
        assert_eq!(fnv1a32_utf16(mixed), format!("{hash:08x}"));
    }
}
