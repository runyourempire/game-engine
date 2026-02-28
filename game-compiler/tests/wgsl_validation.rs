//! WGSL validation tests — parse every generated shader through naga to catch
//! codegen bugs that string-matching tests miss.

use std::fs;
use std::path::Path;

/// Parse WGSL through naga and return any errors.
fn validate_wgsl(wgsl: &str, name: &str) -> Result<(), String> {
    let result = naga::front::wgsl::parse_str(wgsl);
    match result {
        Ok(_module) => Ok(()),
        Err(e) => Err(format!("{name}: naga WGSL parse error:\n{e}")),
    }
}

/// Compile a .game source and validate the generated WGSL.
fn compile_and_validate(source: &str, name: &str) {
    let output = game_compiler::compile_full(source)
        .unwrap_or_else(|e| panic!("{name}: compilation failed: {e}"));

    if let Err(e) = validate_wgsl(&output.wgsl, name) {
        // Print the WGSL with line numbers for debugging
        eprintln!("\n--- Generated WGSL for {name} ---");
        for (i, line) in output.wgsl.lines().enumerate() {
            eprintln!("{:4} | {}", i + 1, line);
        }
        eprintln!("--- End WGSL ---\n");
        panic!("{e}");
    }
}

#[test]
fn validate_basic_circle() {
    compile_and_validate(
        r#"cinematic { layer { fn: circle(0.3) | glow(2.0) } }"#,
        "basic_circle",
    );
}

#[test]
fn validate_audio_reactive() {
    compile_and_validate(
        r#"cinematic {
            layer {
                fn: circle(radius) | glow(intensity)
                radius: 0.3 ~ audio.bass * 0.2
                intensity: 2.0 ~ audio.energy * 3.0
            }
        }"#,
        "audio_reactive",
    );
}

#[test]
fn validate_multi_layer() {
    compile_and_validate(
        r#"cinematic {
            layer a { fn: circle(0.3) | glow(2.0) }
            layer b { fn: box(0.2, 0.2) | glow(1.0) }
        }"#,
        "multi_layer",
    );
}

#[test]
fn validate_raymarch() {
    compile_and_validate(
        r#"cinematic {
            layer terrain {
                fn: fbm(p * 2.0, octaves: 6, persistence: 0.5)
                  | shade(albedo: gold)
            }
            lens {
                mode: raymarch
                camera: orbit(radius: 4.0, height: 2.0, speed: 0.05)
            }
        }"#,
        "raymarch",
    );
}

#[test]
fn validate_spectrum() {
    compile_and_validate(
        r#"cinematic {
            layer {
                fn: spectrum(bass, mid, treble)
                bass: 0.0 ~ audio.bass * 1.5
                mid: 0.0 ~ audio.mid * 1.5
                treble: 0.0 ~ audio.treble * 1.5
            }
        }"#,
        "spectrum",
    );
}

#[test]
fn validate_postprocess_chain() {
    compile_and_validate(
        r#"cinematic {
            layer {
                fn: circle(0.3) | glow(3.0) | bloom(0.5, 1.5) | vignette(0.3) | grain(0.02)
            }
        }"#,
        "postprocess",
    );
}

#[test]
fn validate_domain_operations() {
    compile_and_validate(
        r#"cinematic {
            layer { fn: repeat(0.5) | circle(0.1) | glow(2.0) }
        }"#,
        "repeat",
    );
    compile_and_validate(
        r#"cinematic {
            layer { fn: mirror("x") | circle(0.3) | glow(2.0) }
        }"#,
        "mirror",
    );
    compile_and_validate(
        r#"cinematic {
            layer { fn: scale(2.0) | circle(0.3) | glow(2.0) }
        }"#,
        "scale",
    );
    compile_and_validate(
        r#"cinematic {
            layer { fn: rotate(1.57) | circle(0.3) | glow(2.0) }
        }"#,
        "rotate",
    );
    compile_and_validate(
        r#"cinematic {
            layer { fn: twist(2.0) | circle(0.3) | glow(2.0) }
        }"#,
        "twist",
    );
}

#[test]
fn validate_noise_generators() {
    compile_and_validate(
        r#"cinematic { layer { fn: simplex(3.0) | glow(2.0) } }"#,
        "simplex",
    );
    compile_and_validate(
        r#"cinematic { layer { fn: voronoi(5.0) | glow(2.0) } }"#,
        "voronoi",
    );
    compile_and_validate(
        r#"cinematic { layer { fn: fbm(p, octaves: 4, persistence: 0.6) } }"#,
        "fbm",
    );
}

#[test]
fn validate_all_sdf_primitives() {
    compile_and_validate(
        r#"cinematic { layer { fn: sphere(0.3) | glow(2.0) } }"#,
        "sphere",
    );
    compile_and_validate(
        r#"cinematic { layer { fn: ring(0.3, 0.04) | glow(2.0) } }"#,
        "ring",
    );
    compile_and_validate(
        r#"cinematic { layer { fn: box(0.3, 0.2) | glow(2.0) } }"#,
        "box",
    );
    compile_and_validate(
        r#"cinematic { layer { fn: polygon(6.0, 0.3) | glow(2.0) } }"#,
        "polygon",
    );
    compile_and_validate(
        r#"cinematic { layer { fn: star(5.0, 0.4, 0.2) | glow(3.0) } }"#,
        "star",
    );
    compile_and_validate(
        r#"cinematic { layer { fn: line(-0.5, 0.0, 0.5, 0.0, 0.02) | glow(2.0) } }"#,
        "line",
    );
    compile_and_validate(
        r#"cinematic { layer { fn: torus(0.3, 0.05) | glow(2.0) } }"#,
        "torus",
    );
}

#[test]
fn validate_sdf_modifiers() {
    compile_and_validate(
        r#"cinematic { layer { fn: circle(0.3) | onion(0.02) | glow(2.0) } }"#,
        "onion",
    );
    compile_and_validate(
        r#"cinematic { layer { fn: box(0.3, 0.2) | round(0.05) | glow(2.0) } }"#,
        "round",
    );
    compile_and_validate(
        r#"cinematic { layer { fn: circle(0.3) | displace(0.1) | glow(2.0) } }"#,
        "displace",
    );
}

#[test]
fn validate_color_stages() {
    compile_and_validate(
        r#"cinematic { layer { fn: circle(0.3) | glow(2.0) | tint(gold) } }"#,
        "tint",
    );
    compile_and_validate(
        r#"cinematic { layer { fn: gradient(red, blue, "y") } }"#,
        "gradient",
    );
    compile_and_validate(
        r#"cinematic { layer { fn: circle(0.3) | glow(2.0) | tint(gold) | fog(1.5) } }"#,
        "fog",
    );
    compile_and_validate(
        r#"cinematic { layer { fn: circle(0.3) | glow(2.0) | tint(gold) | scanlines(100.0, 0.3) } }"#,
        "scanlines",
    );
    compile_and_validate(
        r#"cinematic { layer { fn: circle(0.3) | glow(2.0) | tint(gold) | invert() } }"#,
        "invert",
    );
}

#[test]
fn validate_define_expansion() {
    compile_and_validate(
        r#"cinematic {
            define my_shape() { circle(0.3) | glow(2.0) }
            layer { fn: my_shape() }
        }"#,
        "define",
    );
}

#[test]
fn validate_smooth_sdf_ops() {
    compile_and_validate(
        r#"cinematic {
            layer { fn: circle(0.3) | smooth_union(0.1) | glow(2.0) }
        }"#,
        "smooth_union",
    );
}

#[test]
fn validate_color_grade() {
    compile_and_validate(
        r#"cinematic {
            layer { fn: circle(0.3) | glow(2.0) | tint(gold) | color_grade(1.1, 0.0, 1.0) }
        }"#,
        "color_grade",
    );
}

#[test]
fn validate_all_examples() {
    let examples_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples");
    if !examples_dir.exists() {
        return;
    }

    for entry in fs::read_dir(examples_dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().map(|e| e == "game").unwrap_or(false) {
            let source = fs::read_to_string(&path).unwrap();
            let name = path.file_stem().unwrap().to_str().unwrap();
            compile_and_validate(&source, &format!("examples/{name}"));
        }
    }
}

#[test]
fn validate_all_presets() {
    let presets_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("presets");
    if !presets_dir.exists() {
        return;
    }

    for entry in fs::read_dir(presets_dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().map(|e| e == "game").unwrap_or(false) {
            let source = fs::read_to_string(&path).unwrap();
            let name = path.file_stem().unwrap().to_str().unwrap();
            compile_and_validate(&source, &format!("presets/{name}"));
        }
    }
}
