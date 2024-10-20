use std::cell::OnceCell;
use std::collections::HashMap;

use toml::Value;

use super::{ObjectData, ObjectField, Program, ThunkData, ValueData};
use crate::ast;

pub(crate) enum ParseError {
    Parser(toml::de::Error),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::Parser(ref e) => write!(f, "{e}"),
        }
    }
}

pub(super) fn parse_toml(program: &mut Program, s: &str) -> Result<ValueData, ParseError> {
    let table = toml::from_str::<Value>(s).map_err(ParseError::Parser)?;
    Ok(convert_to_value(table, program))
}

fn convert_to_value(val: Value, program: &mut Program) -> ValueData {
    match val {
        Value::Integer(v) => ValueData::Number(v as f64),
        Value::Float(v) => ValueData::Number(v),
        Value::Boolean(v) => ValueData::Bool(v),
        Value::String(s) => ValueData::String(s.into()),
        Value::Table(table) => {
            let mut fields = HashMap::new();
            for (k, v) in table {
                let key = program.str_interner.intern(&k);
                let val = convert_to_value(v, program);
                let field = ObjectField {
                    base_env: None,
                    visibility: ast::Visibility::Default,
                    expr: None,
                    thunk: OnceCell::from(program.gc_alloc(ThunkData::new_done(val))),
                };
                fields.insert(key, field);
            }
            ValueData::Object(program.gc_alloc(ObjectData::new_simple(fields)))
        }
        Value::Array(arr) => {
            let mut items = Vec::new();
            for item in arr {
                let value = convert_to_value(item, program);
                items.push(program.gc_alloc(ThunkData::new_done(value)));
            }
            ValueData::Array(program.gc_alloc(items.into_boxed_slice()))
        }
        Value::Datetime(v) => ValueData::String(v.to_string().into()),
    }
}
