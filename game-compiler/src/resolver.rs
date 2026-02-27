//! Module resolver — resolves `import` declarations by loading and parsing
//! external `.game` files, extracting their `define` blocks, and merging them
//! into the importing cinematic.
//!
//! Supports:
//! - `import "path" expose name1, name2` — import specific defines
//! - `import "path" expose ALL` — import all defines from a file
//! - Circular import detection via path tracking
//! - Relative and library path resolution

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::ast::{Cinematic, DefineBlock, ImportDecl};
use crate::error::{GameError, Result};
use crate::lexer;
use crate::parser::Parser;

/// Resolve all imports in a cinematic, loading define blocks from external files.
///
/// `base_dir` is the directory containing the importing file.
/// `lib_dirs` are additional search paths (e.g., stdlib directory).
pub fn resolve_imports(
    cinematic: &mut Cinematic,
    base_dir: &Path,
    lib_dirs: &[PathBuf],
) -> Result<()> {
    let mut visited = HashSet::new();
    resolve_imports_inner(cinematic, base_dir, lib_dirs, &mut visited)
}

fn resolve_imports_inner(
    cinematic: &mut Cinematic,
    base_dir: &Path,
    lib_dirs: &[PathBuf],
    visited: &mut HashSet<PathBuf>,
) -> Result<()> {
    let imports: Vec<ImportDecl> = cinematic.imports.drain(..).collect();

    for import in imports {
        let resolved_path = resolve_path(&import.path, base_dir, lib_dirs)?;

        // Circular import detection
        let canonical = resolved_path
            .canonicalize()
            .unwrap_or_else(|_| resolved_path.clone());
        if !visited.insert(canonical.clone()) {
            return Err(GameError::parse(&format!(
                "circular import detected: '{}'",
                import.path
            )));
        }

        // Load and parse the imported file
        let source = fs::read_to_string(&resolved_path).map_err(|e| {
            GameError::parse(&format!(
                "cannot read imported file '{}': {e}",
                import.path
            ))
        })?;

        let tokens = lexer::lex(&source)?;
        let mut parser = Parser::new(tokens);
        let mut imported = parser.parse()?;

        // Recursively resolve imports in the imported file
        let import_dir = resolved_path
            .parent()
            .unwrap_or(base_dir)
            .to_path_buf();
        resolve_imports_inner(&mut imported, &import_dir, lib_dirs, visited)?;

        // Extract requested defines
        let defines = extract_defines(&imported, &import)?;
        cinematic.defines.extend(defines);

        // Allow re-visiting from other import chains
        visited.remove(&canonical);
    }

    Ok(())
}

/// Resolve an import path to an actual file path.
fn resolve_path(
    import_path: &str,
    base_dir: &Path,
    lib_dirs: &[PathBuf],
) -> Result<PathBuf> {
    // Try relative to base_dir first
    let relative = base_dir.join(import_path);
    if relative.exists() {
        return Ok(relative);
    }

    // Try with .game extension
    let with_ext = base_dir.join(format!("{import_path}.game"));
    if with_ext.exists() {
        return Ok(with_ext);
    }

    // Search lib_dirs
    for lib_dir in lib_dirs {
        let lib_path = lib_dir.join(import_path);
        if lib_path.exists() {
            return Ok(lib_path);
        }
        let lib_with_ext = lib_dir.join(format!("{import_path}.game"));
        if lib_with_ext.exists() {
            return Ok(lib_with_ext);
        }
    }

    Err(GameError::parse(&format!(
        "cannot resolve import '{}' — file not found",
        import_path
    )))
}

/// Extract the requested defines from an imported cinematic.
fn extract_defines(
    imported: &Cinematic,
    import: &ImportDecl,
) -> Result<Vec<DefineBlock>> {
    // ALL imports everything
    if import.names.len() == 1 && import.names[0] == "ALL" {
        return Ok(imported.defines.clone());
    }

    let mut result = Vec::new();
    for name in &import.names {
        let found = imported
            .defines
            .iter()
            .find(|d| d.name == *name);

        match found {
            Some(define) => result.push(define.clone()),
            None => {
                return Err(GameError::parse(&format!(
                    "import '{}' does not define '{name}'",
                    import.path
                )));
            }
        }
    }

    Ok(result)
}
