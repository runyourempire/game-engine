use std::fs;
use std::path::Path;

fn compile_all_in_dir(dir: &Path) {
    assert!(dir.is_dir(), "{} is not a directory", dir.display());

    let entries: Vec<_> = fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()))
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .map(|ext| ext == "game")
                .unwrap_or(false)
        })
        .collect();

    assert!(!entries.is_empty(), "no .game files found in {}", dir.display());

    for entry in &entries {
        let path = entry.path();
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));

        let output = game_compiler::compile_full(&source)
            .unwrap_or_else(|e| panic!("{} failed to compile: {e}", path.display()));

        assert!(
            !output.wgsl.is_empty(),
            "{} produced empty WGSL",
            path.display()
        );

        assert!(
            output.wgsl.contains("fn fs_main"),
            "{} WGSL missing fragment shader entry point",
            path.display()
        );
    }

    eprintln!("  compiled {} files from {}", entries.len(), dir.display());
}

#[test]
fn all_examples_compile() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples");
    compile_all_in_dir(&dir);
}

#[test]
fn all_presets_compile() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("game-compiler should have a parent dir")
        .join("presets");
    compile_all_in_dir(&dir);
}
