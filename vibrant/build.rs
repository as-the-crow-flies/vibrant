//! Resolves `use` imports in every root shader (a `.wgsl` file with an entry
//! point that no other file imports), validates it with naga, and embeds the
//! flattened sources for `renderer::wgsl::source`.

#[path = "build/shader.rs"]
mod shader;

use std::collections::HashSet;
use std::fmt::Write;
use std::path::{Path, PathBuf};

fn main() {
    println!("cargo::rerun-if-changed=src");
    println!("cargo::rerun-if-changed=build");

    let root = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("src");
    let mut files = Vec::new();
    collect(&root, Path::new(""), &mut files);
    files.sort();

    let mut errors = Vec::new();
    let mut imported = HashSet::new();
    for path in &files {
        match shader::imports(&root, path) {
            Ok(imports) => imported.extend(imports),
            Err(error) => errors.push(error),
        }
    }

    let mut shaders = String::new();
    for path in files.iter().filter(|path| !imported.contains(*path)) {
        let flat = match shader::flatten(&root, path) {
            Ok(flat) => flat,
            Err(error) => {
                errors.push(error);
                continue;
            }
        };

        let is_entry = ["@compute", "@vertex", "@fragment"]
            .iter()
            .any(|stage| flat.source.contains(stage));
        if !is_entry {
            continue;
        }

        match flat.validate() {
            Ok(()) => {
                let _ = writeln!(shaders, "    ({path:?}, {:?}),", flat.source);
            }
            Err(error) => errors.push(error),
        }
    }

    if !errors.is_empty() {
        eprintln!("\n{}", errors.join("\n"));
        eprintln!("{} shader(s) failed to compile", errors.len());
        std::process::exit(1);
    }

    let generated = format!("pub(crate) static SHADERS: &[(&str, &str)] = &[\n{shaders}];\n");
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("shaders.rs");
    std::fs::write(out, generated).unwrap();
}

/// All `.wgsl` files below `dir`, relative to `root`.
fn collect(root: &Path, dir: &Path, files: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(root.join(dir)) else {
        return;
    };
    for entry in entries.flatten() {
        let path = dir.join(entry.file_name());
        if entry.path().is_dir() {
            collect(root, &path, files);
        } else if path.extension().is_some_and(|ext| ext == "wgsl") {
            files.push(path.to_string_lossy().replace('\\', "/"));
        }
    }
}
