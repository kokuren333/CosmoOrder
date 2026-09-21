//! YAML authoring subset: one document, JSON values, no tags/anchors/aliases.

use crate::parsing::MAX_DOCUMENT_BYTES;
use crate::schema::Diagnostic;
use serde_json::{Map, Value};
use yaml_rust2::parser::{Event, Parser};
use yaml_rust2::scanner::{Marker, ScanError, TScalarStyle};

const MAX_DEPTH: usize = 64;

fn scalar(value: String, style: TScalarStyle) -> Value {
    if style == TScalarStyle::Plain {
        if value.is_empty() {
            return Value::Null;
        }
        if let Ok(parsed) = serde_json::from_str::<Value>(&value) {
            if matches!(parsed, Value::Null | Value::Bool(_) | Value::Number(_)) {
                return parsed;
            }
        }
    }
    Value::String(value)
}

fn node(parser: &mut Parser<std::str::Chars<'_>>, depth: usize) -> Result<Value, ScanError> {
    let (event, mark) = parser.next_token()?;
    if depth > MAX_DEPTH {
        return Err(ScanError::new(mark, "YAML nesting exceeds 64 levels"));
    }
    match event {
        Event::Scalar(value, style, 0, None) => Ok(scalar(value, style)),
        Event::SequenceStart(0, None) => {
            let mut values = Vec::new();
            while parser.peek()?.0 != Event::SequenceEnd {
                values.push(node(parser, depth + 1)?);
            }
            parser.next_token()?;
            Ok(Value::Array(values))
        }
        Event::MappingStart(0, None) => {
            let mut values = Map::new();
            while parser.peek()?.0 != Event::MappingEnd {
                let key_mark = parser.peek()?.1;
                let key = node(parser, depth + 1)?;
                let key = key
                    .as_str()
                    .ok_or_else(|| ScanError::new(key_mark, "mapping keys must be strings"))?;
                if key == "<<" || values.contains_key(key) {
                    return Err(ScanError::new(key_mark, "duplicate or merge mapping key"));
                }
                values.insert(key.into(), node(parser, depth + 1)?);
            }
            parser.next_token()?;
            Ok(Value::Object(values))
        }
        _ => Err(ScanError::new(
            mark,
            "anchors, aliases, tags and non-JSON structures are not supported",
        )),
    }
}

fn expect(parser: &mut Parser<std::str::Chars<'_>>, expected: Event) -> Result<(), ScanError> {
    let (event, mark) = parser.next_token()?;
    if event == expected {
        Ok(())
    } else {
        Err(ScanError::new(mark, "expected exactly one YAML document"))
    }
}

pub fn parse_yaml(bytes: &[u8], file: &str) -> Result<Value, Box<Diagnostic>> {
    let diagnostic = |message: String, mark: Option<Marker>| {
        Box::new(Diagnostic {
            code: "OSM_YAML".into(),
            severity: "error".into(),
            file: Some(file.into()),
            line: mark.map(|m| m.line()),
            // yaml-rust2 emits zero-based columns (covered by position tests).
            column: mark.map(|m| m.col() + 1),
            path: String::new(),
            message,
            suggestions: Vec::new(),
        })
    };
    if bytes.len() > MAX_DOCUMENT_BYTES {
        let mut error = diagnostic("YAML exceeds 4 MiB".into(), None);
        error.code = "OSM_INPUT_LIMIT".into();
        return Err(error);
    }
    let input = std::str::from_utf8(bytes).map_err(|e| diagnostic(e.to_string(), None))?;
    let mut parser = Parser::new(input.chars());
    let parse = || -> Result<Value, ScanError> {
        expect(&mut parser, Event::StreamStart)?;
        expect(&mut parser, Event::DocumentStart)?;
        let value = node(&mut parser, 0)?;
        expect(&mut parser, Event::DocumentEnd)?;
        expect(&mut parser, Event::StreamEnd)?;
        Ok(value)
    };
    let mut parse = parse;
    parse().map_err(|e| diagnostic(e.to_string(), Some(*e.marker())))
}
