//! Dynamically typed cell value. Maps 1:1 onto JSON for the wire.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(untagged)]
pub enum Value {
    #[default]
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Text(String),
    List(Vec<Value>),
    Map(BTreeMap<String, Value>),
}

pub type Row = BTreeMap<String, Value>;

impl Value {
    pub fn is_null(&self) -> bool { matches!(self, Value::Null) }
    pub fn as_i64(&self) -> Option<i64> {
        match self { Value::Int(i) => Some(*i), Value::Bool(b) => Some(*b as i64), Value::Float(f) => Some(*f as i64), Value::Text(s) => s.parse().ok(), _ => None }
    }
    pub fn as_f64(&self) -> Option<f64> {
        match self { Value::Int(i) => Some(*i as f64), Value::Float(f) => Some(*f), Value::Text(s) => s.parse().ok(), _ => None }
    }
    pub fn as_str(&self) -> Option<&str> { if let Value::Text(s) = self { Some(s) } else { None } }
    pub fn truthy(&self) -> bool {
        match self { Value::Null => false, Value::Bool(b) => *b, Value::Int(i) => *i != 0, Value::Float(f) => *f != 0.0, Value::Text(s) => !s.is_empty(), Value::List(l) => !l.is_empty(), Value::Map(m) => !m.is_empty() }
    }
    pub fn from_json(j: &serde_json::Value) -> Value {
        use serde_json::Value as J;
        match j {
            J::Null => Value::Null,
            J::Bool(b) => Value::Bool(*b),
            J::Number(n) => n.as_i64().map(Value::Int).unwrap_or_else(|| Value::Float(n.as_f64().unwrap_or(0.0))),
            J::String(s) => Value::Text(s.clone()),
            J::Array(a) => Value::List(a.iter().map(Value::from_json).collect()),
            J::Object(o) => Value::Map(o.iter().map(|(k, v)| (k.clone(), Value::from_json(v))).collect()),
        }
    }
    pub fn to_json(&self) -> serde_json::Value { serde_json::to_value(self).unwrap_or(serde_json::Value::Null) }
}

impl From<i64> for Value { fn from(v: i64) -> Self { Value::Int(v) } }
impl From<f64> for Value { fn from(v: f64) -> Self { Value::Float(v) } }
impl From<bool> for Value { fn from(v: bool) -> Self { Value::Bool(v) } }
impl From<&str> for Value { fn from(v: &str) -> Self { Value::Text(v.into()) } }
impl From<String> for Value { fn from(v: String) -> Self { Value::Text(v) } }
