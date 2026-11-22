// SPDX-License-Identifier: MIT
//! A small JSON reader and writer for ship recipes and ship packs.

use ranger::prelude::*;

#[derive(Clone, Debug)]
pub enum Json {
    Null,
    Bool(bool),
    Num(double),
    Str(string),
    Arr(Vec<Json>),
    Obj(Vec<JsonMember>),
}

#[derive(Clone, Debug)]
pub struct JsonMember {
    pub key: string,
    pub value: Json,
}

impl Json {
    /// The member `key` of an object, or `Json::Null`.
    pub fn get(&self, key: &str) -> Json {
        match self {
            Json::Obj(members) => {
                for m in members.iter() {
                    if m.key == key {
                        return m.value.clone();
                    }
                }
                Json::Null
            }
            _ => Json::Null,
        }
    }

    pub fn has(&self, key: &str) -> bool {
        match self {
            Json::Obj(members) => {
                for m in members.iter() {
                    if m.key == key {
                        return true;
                    }
                }
                false
            }
            _ => false,
        }
    }

    pub fn is_null(&self) -> bool {
        match self {
            Json::Null => true,
            _ => false,
        }
    }

    pub fn str_or(&self, fallback: &str) -> string {
        match self {
            Json::Str(s) => s.clone(),
            _ => fallback.to_string(),
        }
    }

    pub fn num_or(&self, fallback: double) -> double {
        match self {
            Json::Num(n) => *n,
            _ => fallback,
        }
    }

    pub fn bool_or(&self, fallback: bool) -> bool {
        match self {
            Json::Bool(b) => *b,
            Json::Num(n) => *n != 0.0,
            _ => fallback,
        }
    }

    /// The items of an array (none for anything else).
    pub fn items(&self) -> Vec<Json> {
        match self {
            Json::Arr(items) => items.clone(),
            _ => Vec::new(),
        }
    }

    pub fn count(&self) -> int {
        match self {
            Json::Arr(items) => items.len() as int,
            _ => 0,
        }
    }

    /// Item `i` of an array as a number.
    pub fn at_num(&self, i: int, fallback: double) -> double {
        match self {
            Json::Arr(items) => {
                if i >= 0 && i < (items.len() as int) {
                    items[i as usize].num_or(fallback)
                } else {
                    fallback
                }
            }
            _ => fallback,
        }
    }

    pub fn write(&self) -> string {
        let mut out = String::new();
        write_into(self, &mut out);
        out
    }
}

/// `s` as a JSON string literal.
pub fn quote(s: &str) -> string {
    let mut out = String::from("\"");
    for c in s.chars() {
        let code = c as i64;
        if c == '"' {
            out.push_str("\\\"");
        } else if c == '\\' {
            out.push_str("\\\\");
        } else if c == '\n' {
            out.push_str("\\n");
        } else if c == '\r' {
            out.push_str("\\r");
        } else if c == '\t' {
            out.push_str("\\t");
        } else if code < 32 {
            out.push_str(" ");
        } else {
            out.push(c);
        }
    }
    out.push('"');
    out
}

/// A number with at most 4 decimals, integers without a fraction.
pub fn num(n: double) -> string {
    if n == n.floor() && n.abs() < 1.0e15 {
        return format!("{}", n as i64);
    }
    let r = (n * 10000.0).round() / 10000.0;
    if r == r.floor() {
        return format!("{}", r as i64);
    }
    format!("{}", r)
}

fn write_into(v: &Json, out: &mut String) {
    match v {
        Json::Null => out.push_str("null"),
        Json::Bool(b) => {
            if *b {
                out.push_str("true");
            } else {
                out.push_str("false");
            }
        }
        Json::Num(n) => out.push_str(&num(*n)),
        Json::Str(s) => out.push_str(&quote(s)),
        Json::Arr(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_into(item, out);
            }
            out.push(']');
        }
        Json::Obj(members) => {
            out.push('{');
            for (i, m) in members.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push_str(&quote(&m.key));
                out.push(':');
                write_into(&m.value, out);
            }
            out.push('}');
        }
    }
}

/// Parses `text`. A syntax error answers `Json::Null` and sets `error`.
pub struct JsonReader {
    bytes: Vec<u8>,
    pos: int,
    pub error: string,
}

impl JsonReader {
    pub fn new(text: &str) -> JsonReader {
        JsonReader { bytes: text.as_bytes().to_vec(), pos: 0, error: String::new() }
    }

    fn size(&self) -> int {
        self.bytes.len() as int
    }

    fn peek(&self) -> int {
        if self.pos < self.size() {
            self.bytes[self.pos as usize] as int
        } else {
            -1
        }
    }

    fn skip_ws(&mut self) {
        while self.pos < self.size() {
            let c = self.peek();
            if c == 32 || c == 9 || c == 10 || c == 13 {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    fn fail(&mut self, msg: &str) -> Json {
        if self.error.is_empty() {
            self.error = format!("{} at byte {}", msg, self.pos);
        }
        Json::Null
    }

    pub fn parse(&mut self) -> Json {
        let v = self.value();
        self.skip_ws();
        if self.error.is_empty() && self.pos < self.size() {
            return self.fail("trailing text");
        }
        v
    }

    fn value(&mut self) -> Json {
        self.skip_ws();
        let c = self.peek();
        if c == 123 {
            return self.object();
        }
        if c == 91 {
            return self.array();
        }
        if c == 34 {
            let s = self.string_lit();
            return Json::Str(s);
        }
        if c == 116 {
            return self.word("true", Json::Bool(true));
        }
        if c == 102 {
            return self.word("false", Json::Bool(false));
        }
        if c == 110 {
            return self.word("null", Json::Null);
        }
        if c == 45 || (c >= 48 && c <= 57) {
            return self.number();
        }
        self.fail("unexpected character")
    }

    fn word(&mut self, w: &str, v: Json) -> Json {
        let wb = w.as_bytes();
        let n = wb.len() as int;
        if self.pos + n > self.size() {
            return self.fail("unexpected end");
        }
        let mut i: int = 0;
        while i < n {
            if self.bytes[(self.pos + i) as usize] != wb[i as usize] {
                return self.fail("unknown word");
            }
            i += 1;
        }
        self.pos += n;
        v
    }

    fn number(&mut self) -> Json {
        let start = self.pos;
        while self.pos < self.size() {
            let c = self.peek();
            if (c >= 48 && c <= 57) || c == 45 || c == 43 || c == 46 || c == 101 || c == 69 {
                self.pos += 1;
            } else {
                break;
            }
        }
        let text = String::from_utf8_lossy(&self.bytes[(start as usize)..(self.pos as usize)]).to_string();
        match text.parse::<f64>() {
            Ok(n) => Json::Num(n),
            Err(_) => self.fail("bad number"),
        }
    }

    fn string_lit(&mut self) -> string {
        // the opening quote
        self.pos += 1;
        let start = self.pos;
        let mut plain = true;
        while self.pos < self.size() {
            let c = self.peek();
            if c == 34 {
                break;
            }
            if c == 92 {
                plain = false;
                self.pos += 1;
            }
            self.pos += 1;
        }
        let end = self.pos;
        if self.pos >= self.size() {
            self.fail("unterminated string");
            return String::new();
        }
        self.pos += 1;
        let raw = String::from_utf8_lossy(&self.bytes[(start as usize)..(end as usize)]).to_string();
        if plain {
            return raw;
        }
        // escapes: the common ones; \uXXXX becomes a space (recipes are names and colours)
        let mut out = String::new();
        let mut esc = false;
        let mut skip = 0;
        for c in raw.chars() {
            if skip > 0 {
                skip -= 1;
                continue;
            }
            if esc {
                esc = false;
                if c == 'n' {
                    out.push('\n');
                } else if c == 't' {
                    out.push('\t');
                } else if c == 'r' {
                    out.push('\r');
                } else if c == 'u' {
                    out.push(' ');
                    skip = 4;
                } else {
                    out.push(c);
                }
            } else if c == '\\' {
                esc = true;
            } else {
                out.push(c);
            }
        }
        out
    }

    fn array(&mut self) -> Json {
        self.pos += 1;
        let mut items: Vec<Json> = Vec::new();
        self.skip_ws();
        if self.peek() == 93 {
            self.pos += 1;
            return Json::Arr(items);
        }
        while self.error.is_empty() {
            let v = self.value();
            items.push(v);
            self.skip_ws();
            let c = self.peek();
            if c == 44 {
                self.pos += 1;
            } else if c == 93 {
                self.pos += 1;
                break;
            } else {
                return self.fail("expected , or ]");
            }
        }
        Json::Arr(items)
    }

    fn object(&mut self) -> Json {
        self.pos += 1;
        let mut members: Vec<JsonMember> = Vec::new();
        self.skip_ws();
        if self.peek() == 125 {
            self.pos += 1;
            return Json::Obj(members);
        }
        while self.error.is_empty() {
            self.skip_ws();
            if self.peek() != 34 {
                return self.fail("expected a key");
            }
            let key = self.string_lit();
            self.skip_ws();
            if self.peek() != 58 {
                return self.fail("expected :");
            }
            self.pos += 1;
            let v = self.value();
            members.push(JsonMember { key: key, value: v });
            self.skip_ws();
            let c = self.peek();
            if c == 44 {
                self.pos += 1;
            } else if c == 125 {
                self.pos += 1;
                break;
            } else {
                return self.fail("expected , or }");
            }
        }
        Json::Obj(members)
    }
}

pub fn parse(text: &str) -> (Json, string) {
    let mut r = JsonReader::new(text);
    let v = r.parse();
    let e = r.error.clone();
    (v, e)
}
