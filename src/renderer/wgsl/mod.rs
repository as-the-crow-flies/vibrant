use std::fmt::Display;

include!(concat!(env!("OUT_DIR"), "/shaders.rs"));

/// The shader at `path` (relative to `src/`) with its `use` imports inlined by
/// build.rs.
pub fn source(path: &str) -> &'static str {
    SHADERS
        .iter()
        .find(|(shader, _)| *shader == path)
        .map(|(_, source)| *source)
        .unwrap_or_else(|| panic!("no root shader {path}"))
}

/// Replaces the value of `const name: T = value;` in `source`.
pub fn define(source: &str, name: &str, value: &dyn Display) -> String {
    let start = source
        .find(&format!("const {name}:"))
        .unwrap_or_else(|| panic!("no const {name} to define"));
    let value_start = start + source[start..].find('=').unwrap() + 1;
    let value_end = value_start + source[value_start..].find(';').unwrap();

    format!("{} {value}{}", &source[..value_start], &source[value_end..])
}
