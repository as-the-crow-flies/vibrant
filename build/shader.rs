//! Shader preprocessing for `build.rs`.
//!
//! `use name;` lines are replaced by the contents of `name.wgsl`, looked up next
//! to the importing file first, then in [`LIB`]. Every file is included at most
//! once. The result is validated with naga, and errors point at the original
//! file and line.

use std::collections::HashSet;
use std::fmt::Write;
use std::path::Path;

/// Shared shader library, relative to `src/`.
pub const LIB: &str = "renderer/wgsl";

/// A shader with all imports inlined.
pub struct Flat {
    pub source: String,
    /// Every file that ended up in `source`, relative to `src/`.
    files: Vec<String>,
    /// For every line of `source`: (index into `files`, 1-based line number).
    lines: Vec<(usize, u32)>,
}

/// Inline all imports of `path` (relative to `root`, the `src/` directory).
pub fn flatten(root: &Path, path: &str) -> Result<Flat, String> {
    let mut flat = Flat {
        source: String::new(),
        files: Vec::new(),
        lines: Vec::new(),
    };
    include(root, path, &mut flat, &mut HashSet::new())?;
    Ok(flat)
}

/// The files `path` imports directly.
pub fn imports(root: &Path, path: &str) -> Result<Vec<String>, String> {
    let text = read(root, path)?;
    text.lines()
        .enumerate()
        .filter_map(|(i, line)| import(line).map(|name| (i as u32 + 1, name)))
        .map(|(number, name)| resolve(root, path, name, number))
        .collect()
}

fn include(
    root: &Path,
    path: &str,
    flat: &mut Flat,
    seen: &mut HashSet<String>,
) -> Result<(), String> {
    if !seen.insert(path.to_string()) {
        return Ok(());
    }

    let text = read(root, path)?;
    let index = flat.files.len();
    flat.files.push(path.to_string());

    for (i, line) in text.lines().enumerate() {
        let number = i as u32 + 1;

        if let Some(name) = import(line) {
            let resolved = resolve(root, path, name, number)?;
            include(root, &resolved, flat, seen)?;
            // Keep the import line (as a blank line) so line numbers stay in sync.
            flat.source.push('\n');
        } else {
            flat.source.push_str(line);
            flat.source.push('\n');
        }
        flat.lines.push((index, number));
    }

    Ok(())
}

fn read(root: &Path, path: &str) -> Result<String, String> {
    std::fs::read_to_string(root.join(path))
        .map_err(|error| format!("error: cannot read {path}: {error}"))
}

/// `use name;` -> `name`.
fn import(line: &str) -> Option<&str> {
    let rest = line.trim().strip_prefix("use ")?;
    Some(rest.strip_suffix(';').unwrap_or(rest).trim())
}

/// Find an import next to the importing file, or else in [`LIB`].
fn resolve(root: &Path, from: &str, name: &str, number: u32) -> Result<String, String> {
    let file = format!("{name}.wgsl");
    let dir = Path::new(from).parent().unwrap_or(Path::new(""));

    let mut found: Vec<String> = [dir.join(&file), Path::new(LIB).join(&file)]
        .into_iter()
        .map(|candidate| normalize(&candidate.to_string_lossy()))
        .filter(|candidate| root.join(candidate).is_file())
        .collect();
    found.dedup();

    match found.as_slice() {
        [path] => Ok(path.clone()),
        [] => Err(format!(
            "error: cannot find `use {name}`\n  --> {from}:{number}\n   = note: looked next to {from} and in {LIB}/"
        )),
        [local, lib] => Err(format!(
            "error: `use {name}` is ambiguous\n  --> {from}:{number}\n   = note: both {local} and {lib} exist"
        )),
        _ => unreachable!(),
    }
}

/// `a/b/../c/./d.wgsl` -> `a/c/d.wgsl`, always with `/`.
fn normalize(path: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for part in path.split(['/', '\\']) {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            part => parts.push(part),
        }
    }
    parts.join("/")
}

impl Flat {
    /// Parse and validate with naga.
    pub fn validate(&self) -> Result<(), String> {
        let module = naga::front::wgsl::parse_str(&self.source).map_err(|error| {
            self.report(
                error.message(),
                error
                    .labels()
                    .map(|(span, label)| (span, label.to_string())),
                error.notes().map(str::to_string),
            )
        })?;

        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::SUBGROUP,
        )
        .subgroup_stages(naga::valid::ShaderStages::all())
        .subgroup_operations(naga::valid::SubgroupOperationSet::all())
        .validate(&module)
        .map_err(|error| {
            let mut notes = Vec::new();
            let mut source = std::error::Error::source(error.as_inner());
            while let Some(next) = source {
                notes.push(next.to_string());
                source = next.source();
            }
            self.report(
                &error.as_inner().to_string(),
                error.spans().cloned(),
                notes.into_iter(),
            )
        })?;

        Ok(())
    }

    /// Format an error like rustc does, pointing at the original file and line.
    fn report(
        &self,
        message: &str,
        labels: impl Iterator<Item = (naga::Span, String)>,
        notes: impl Iterator<Item = String>,
    ) -> String {
        let mut out = format!("error: {message}\n");
        let mut any_label = false;

        for (span, label) in labels {
            let Some(range) = span.to_range() else {
                continue;
            };
            any_label = true;

            let line_index = self.source[..range.start].matches('\n').count();
            let line_start = self.source[..range.start].rfind('\n').map_or(0, |i| i + 1);
            let line_text = self.source[line_start..].lines().next().unwrap_or("");
            let column = range.start - line_start;
            let width =
                (range.end - range.start).clamp(1, line_text.len().saturating_sub(column).max(1));

            let (file, line) = self.lines.get(line_index).copied().unwrap_or((0, 0));
            let file = &self.files[file];
            let gutter = " ".repeat(line.to_string().len());

            let _ = writeln!(out, "{gutter}--> src/{file}:{line}:{}", column + 1);
            let _ = writeln!(out, "{gutter} |");
            let _ = writeln!(out, "{line} | {line_text}");
            let _ = writeln!(
                out,
                "{gutter} | {}{} {label}",
                " ".repeat(column),
                "^".repeat(width)
            );
        }

        if !any_label {
            let _ = writeln!(out, "  --> src/{}", self.files[0]);
        }
        for note in notes {
            let _ = writeln!(out, "   = note: {note}");
        }
        out
    }
}
