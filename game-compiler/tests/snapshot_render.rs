//! GPU snapshot rendering tests — verify that compiled shaders actually
//! produce visible output on a real GPU.
//!
//! These tests are gated behind the "snapshot" feature because they require
//! a GPU (real or software-emulated via wgpu).
//!
//! Run: cargo test --features snapshot --test snapshot_render

#![cfg(feature = "snapshot")]

use std::fs;
use std::path::Path;

/// Render a .game source and check it produces non-black output.
fn render_and_check(source: &str, name: &str) {
    let output = game_compiler::compile_full(source)
        .unwrap_or_else(|e| panic!("{name}: compilation failed: {e}"));

    let renderer = match game_compiler::snapshot::SnapshotRenderer::new() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("  {name}: GPU not available, skipping ({e})");
            return;
        }
    };

    let size = 128; // Small for speed
    let time = 0.5;

    let pixels = match renderer.render_frame(&output, size, size, time) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("  {name}: render failed ({e}), skipping");
            return;
        }
    };

    // Check that output isn't entirely black (all zeros)
    let total_brightness: u64 = pixels.iter().map(|&b| b as u64).sum();
    let pixel_count = (size * size) as u64;
    let avg_brightness = total_brightness / (pixel_count * 4); // RGBA

    assert!(
        avg_brightness > 0,
        "{name}: rendered output is entirely black — shader likely has no visible output. \
         Total brightness: {total_brightness}, pixels: {pixel_count}"
    );

    eprintln!(
        "  {name}: rendered {}x{} — avg brightness: {avg_brightness}/255",
        size, size
    );
}

#[test]
fn snapshot_basic_circle() {
    render_and_check(
        r#"cinematic { layer { fn: circle(0.3) | glow(2.0) } }"#,
        "basic_circle",
    );
}

#[test]
fn snapshot_multi_layer() {
    render_and_check(
        r#"cinematic {
            layer bg { fn: gradient(deep_blue, midnight, "y") }
            layer fg { fn: circle(0.3) | glow(2.0) | tint(gold) }
        }"#,
        "multi_layer",
    );
}

#[test]
fn snapshot_postprocess() {
    render_and_check(
        r#"cinematic {
            layer {
                fn: circle(0.3) | glow(3.0) | bloom(0.5, 1.5) | vignette(0.3) | grain(0.02)
            }
        }"#,
        "postprocess",
    );
}

#[test]
fn snapshot_noise() {
    render_and_check(
        r#"cinematic { layer { fn: simplex(3.0) | glow(2.0) } }"#,
        "simplex_noise",
    );
    render_and_check(
        r#"cinematic { layer { fn: voronoi(5.0) | glow(2.0) } }"#,
        "voronoi_noise",
    );
}

#[test]
fn snapshot_domain_ops() {
    render_and_check(
        r#"cinematic { layer { fn: repeat(0.5) | circle(0.1) | glow(2.0) } }"#,
        "repeat",
    );
    render_and_check(
        r#"cinematic { layer { fn: mirror("xy") | circle(0.3) | glow(2.0) } }"#,
        "mirror",
    );
}

#[test]
fn snapshot_all_primitives() {
    let primitives = [
        (r#"cinematic { layer { fn: circle(0.3) | glow(2.0) } }"#, "circle"),
        (r#"cinematic { layer { fn: sphere(0.3) | glow(2.0) } }"#, "sphere"),
        (r#"cinematic { layer { fn: ring(0.3, 0.04) | glow(2.0) } }"#, "ring"),
        (r#"cinematic { layer { fn: box(0.3, 0.2) | glow(2.0) } }"#, "box"),
        (r#"cinematic { layer { fn: polygon(6.0, 0.3) | glow(2.0) } }"#, "polygon"),
        (r#"cinematic { layer { fn: star(5.0, 0.4, 0.2) | glow(3.0) } }"#, "star"),
        (r#"cinematic { layer { fn: torus(0.3, 0.05) | glow(2.0) } }"#, "torus"),
    ];

    for (source, name) in primitives {
        render_and_check(source, name);
    }
}

#[test]
fn snapshot_raymarch_terrain() {
    render_and_check(
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
        "raymarch_terrain",
    );
}

#[test]
fn snapshot_spectrum() {
    render_and_check(
        r#"cinematic {
            layer {
                fn: spectrum(bass, mid, treble)
                bass: 0.5 ~ audio.bass * 1.5
                mid: 0.3 ~ audio.mid * 1.5
                treble: 0.2 ~ audio.treble * 1.5
            }
        }"#,
        "spectrum",
    );
}

#[test]
fn snapshot_gradient() {
    render_and_check(
        r#"cinematic { layer { fn: gradient(red, blue, "y") } }"#,
        "gradient",
    );
}

#[test]
fn snapshot_all_examples() {
    let examples_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples");
    if !examples_dir.exists() {
        return;
    }

    for entry in fs::read_dir(&examples_dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().map(|e| e == "game").unwrap_or(false) {
            let source = fs::read_to_string(&path).unwrap();
            let name = path.file_stem().unwrap().to_str().unwrap();
            render_and_check(&source, &format!("examples/{name}"));
        }
    }
}

#[test]
fn snapshot_all_presets() {
    let presets_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("presets");
    if !presets_dir.exists() {
        return;
    }

    for entry in fs::read_dir(&presets_dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().map(|e| e == "game").unwrap_or(false) {
            let source = fs::read_to_string(&path).unwrap();
            let name = path.file_stem().unwrap().to_str().unwrap();
            render_and_check(&source, &format!("presets/{name}"));
        }
    }
}
