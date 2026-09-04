use serde_json::Value;

// CamillaDSP reads YAML from disk, so a config spliced as JSON is written back out through
// here. Only the shapes serde_json produces are handled — no anchors, no tags, no folding.
pub fn to_yaml(value: &Value) -> String {
    let mut out = String::new();
    write_node(&mut out, value, 0, false);
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

fn write_node(out: &mut String, value: &Value, indent: usize, inline_start: bool) {
    match value {
        Value::Object(map) if !map.is_empty() => {
            for (i, (k, v)) in map.iter().enumerate() {
                let pad = if inline_start && i == 0 {
                    String::new()
                } else {
                    " ".repeat(indent)
                };
                out.push_str(&format!("{}{}:", pad, key(k)));
                write_child(out, v, indent);
            }
        }
        Value::Array(items) if !items.is_empty() => {
            for (i, v) in items.iter().enumerate() {
                let pad = if inline_start && i == 0 {
                    String::new()
                } else {
                    " ".repeat(indent)
                };
                out.push_str(&format!("{}- ", pad));
                match v {
                    Value::Object(_) | Value::Array(_) => write_node(out, v, indent + 2, true),
                    _ => {
                        out.push_str(&scalar(v));
                        out.push('\n');
                    }
                }
            }
        }
        _ => {
            out.push_str(&scalar(value));
            out.push('\n');
        }
    }
}

fn write_child(out: &mut String, v: &Value, indent: usize) {
    match v {
        Value::Object(m) if m.is_empty() => out.push_str(" {}\n"),
        Value::Array(a) if a.is_empty() => out.push_str(" []\n"),
        Value::Object(_) | Value::Array(_) => {
            out.push('\n');
            write_node(out, v, indent + 2, false);
        }
        _ => {
            out.push(' ');
            out.push_str(&scalar(v));
            out.push('\n');
        }
    }
}

fn key(k: &str) -> String {
    let plain = !k.is_empty()
        && k.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.');
    if plain {
        k.to_string()
    } else {
        quote(k)
    }
}

fn scalar(v: &Value) -> String {
    match v {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => quote(s),
        Value::Object(_) => "{}".to_string(),
        Value::Array(_) => "[]".to_string(),
    }
}

fn quote(s: &str) -> String {
    let escaped = s
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n");
    format!("\"{}\"", escaped)
}
