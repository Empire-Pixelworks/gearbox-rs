use std::str::FromStr;
use uuid::Uuid;

pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Uuid(Uuid),
}

impl Value {
    pub fn from_str(value: &str) -> Value {
        if value.eq_ignore_ascii_case("null") {
            return Value::Null
        }
        if let Ok(bool) = bool::from_str(value) {
            return Value::Bool(bool)
        }
        if let Ok(int) = i64::from_str(value) {
            return Value::Int(int)
        }
        if let Ok(float) = f64::from_str(value) {
            return Value::Float(float)
        }
        if let Ok(uuid) = Uuid::from_str(value) {
            return Value::Uuid(uuid)
        }

        Value::String(value.to_string())
    }
}