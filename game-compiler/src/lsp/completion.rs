use tower_lsp::lsp_types::*;

/// Build the full list of completion items for .game files.
pub fn all_completions() -> Vec<CompletionItem> {
    let mut items = Vec::new();
    items.extend(stage_completions());
    items.extend(builtin_completions());
    items.extend(color_completions());
    items.extend(keyword_completions());
    items
}

/// Completion items for all pipeline stage names.
pub fn stage_completions() -> Vec<CompletionItem> {
    let stages: &[(&str, &str, &str)] = &[
        // SDF primitives
        ("circle", "circle(${1:0.5})", "SDF circle primitive"),
        ("sphere", "sphere(${1:0.5})", "SDF sphere primitive"),
        ("ring", "ring(${1:0.3}, ${2:0.04})", "SDF ring (annulus)"),
        ("box", "box(${1:0.5}, ${2:0.5})", "SDF box (rectangle)"),
        ("torus", "torus(${1:0.3}, ${2:0.05})", "SDF torus"),
        ("cylinder", "cylinder(${1:0.3}, ${2:0.5})", "SDF cylinder"),
        ("plane", "plane()", "SDF infinite plane"),
        ("line", "line(${1:-0.5}, ${2:0.0}, ${3:0.5}, ${4:0.0}, ${5:0.02})", "SDF line segment"),
        ("polygon", "polygon(${1:6}, ${2:0.3})", "SDF regular polygon"),
        ("star", "star(${1:5}, ${2:0.4}, ${3:0.2})", "SDF star shape"),
        // Noise
        ("fbm", "fbm(${1:p})", "Fractal Brownian Motion noise"),
        ("simplex", "simplex(${1:1.0})", "Simplex noise"),
        ("voronoi", "voronoi(${1:1.0})", "Voronoi cell noise"),
        ("noise", "noise(${1:1.0})", "Basic 2D noise"),
        ("curl_noise", "curl_noise(${1:p})", "Curl noise field"),
        ("concentric_waves", "concentric_waves(${1:p})", "Concentric wave ripples"),
        // Domain operations
        ("translate", "translate(${1:0.0}, ${2:0.0})", "Translate coordinates"),
        ("rotate", "rotate(${1:0.0})", "Rotate coordinates"),
        ("scale", "scale(${1:1.0})", "Scale coordinates"),
        ("repeat", "repeat(${1:1.0})", "Tile/repeat space"),
        ("mirror", "mirror(${1:\"xy\"})", "Mirror coordinates"),
        ("twist", "twist(${1:1.0})", "Twist coordinates"),
        // SDF modifiers
        ("displace", "displace(${1:0.1})", "Displace SDF with noise"),
        ("round", "round(${1:0.05})", "Round SDF edges"),
        ("onion", "onion(${1:0.02})", "Hollow out SDF (shell)"),
        ("mask_arc", "mask_arc(${1:6.283})", "Arc mask for SDF"),
        ("threshold", "threshold(${1:0.5})", "Step threshold"),
        // Smooth booleans
        ("smooth_union", "smooth_union(${1:0.1})", "Smooth SDF union"),
        ("smooth_subtract", "smooth_subtract(${1:0.1})", "Smooth SDF subtraction"),
        ("smooth_intersect", "smooth_intersect(${1:0.1})", "Smooth SDF intersection"),
        // Glow
        ("glow", "glow(${1:2.0})", "Apply glow to SDF"),
        // Color / shading
        ("shade", "shade(albedo: ${1:vec3f(0.8)})", "Material shading"),
        ("emissive", "emissive()", "Emissive color output"),
        ("colormap", "colormap()", "SDF-to-color mapping"),
        ("spectrum", "spectrum(${1:audio.bass}, ${2:audio.mid}, ${3:audio.treble})", "Audio spectrum visualizer"),
        ("tint", "tint(${1:gold})", "Apply color tint"),
        ("gradient", "gradient(${1:black}, ${2:white})", "Color gradient"),
        ("particles", "particles(count: ${1:100})", "Particle field"),
        // Post-processing
        ("bloom", "bloom(${1:0.6}, ${2:1.5})", "Bloom post-processing"),
        ("chromatic", "chromatic(${1:0.5})", "Chromatic aberration"),
        ("vignette", "vignette(${1:0.3})", "Vignette darkening"),
        ("grain", "grain(${1:0.02})", "Film grain noise"),
        ("fog", "fog(${1:1.0})", "Distance fog"),
        ("glitch", "glitch(${1:0.5})", "Glitch distortion"),
        ("scanlines", "scanlines(${1:100}, ${2:0.3})", "CRT scanlines"),
        ("tonemap", "tonemap(${1:1.0})", "Tonemapping"),
        ("invert", "invert()", "Invert colors"),
        ("saturate_color", "saturate_color(${1:1.5})", "Saturation control"),
        ("iridescent", "iridescent(${1:0.3})", "Iridescent color shift"),
        ("color_grade", "color_grade(${1:1.0}, ${2:0.0}, ${3:1.0})", "Color grading"),
    ];

    stages
        .iter()
        .map(|(label, snippet, detail)| CompletionItem {
            label: label.to_string(),
            kind: Some(CompletionItemKind::FUNCTION),
            detail: Some(detail.to_string()),
            insert_text: Some(snippet.to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        })
        .collect()
}

/// Completion items for built-in identifiers.
fn builtin_completions() -> Vec<CompletionItem> {
    let builtins: &[(&str, &str)] = &[
        ("time", "Elapsed time in seconds"),
        ("p", "Current 2D position (aspect-corrected)"),
        ("uv", "UV coordinates (-1..1)"),
        ("height", "SDF distance mapped to 0..1"),
        ("pi", "Mathematical constant pi"),
        ("tau", "Mathematical constant tau (2*pi)"),
        ("e", "Euler's number"),
        ("phi", "Golden ratio"),
        ("audio.bass", "Audio bass frequency band"),
        ("audio.mid", "Audio mid frequency band"),
        ("audio.treble", "Audio treble frequency band"),
        ("audio.energy", "Audio total energy"),
        ("audio.beat", "Audio beat detection signal"),
        ("mouse.x", "Mouse X position (normalized)"),
        ("mouse.y", "Mouse Y position (normalized)"),
    ];

    builtins
        .iter()
        .map(|(label, detail)| CompletionItem {
            label: label.to_string(),
            kind: Some(CompletionItemKind::VARIABLE),
            detail: Some(detail.to_string()),
            ..Default::default()
        })
        .collect()
}

/// Completion items for named colors.
fn color_completions() -> Vec<CompletionItem> {
    let colors: &[(&str, &str)] = &[
        ("black", "vec3f(0.0, 0.0, 0.0)"),
        ("white", "vec3f(1.0, 1.0, 1.0)"),
        ("red", "vec3f(1.0, 0.0, 0.0)"),
        ("green", "vec3f(0.0, 1.0, 0.0)"),
        ("blue", "vec3f(0.0, 0.0, 1.0)"),
        ("gold", "vec3f(1.0, 0.84, 0.0)"),
        ("midnight", "Deep blue/black"),
        ("obsidian", "Very dark grey/black"),
        ("ember", "Warm orange-red"),
        ("cyan", "vec3f(0.0, 1.0, 1.0)"),
        ("ivory", "Off-white warm"),
        ("frost", "Cool light blue"),
        ("orange", "vec3f(1.0, 0.5, 0.0)"),
        ("deep_blue", "Rich saturated blue"),
        ("ash", "Light grey"),
        ("charcoal", "Dark grey"),
        ("plasma", "Vibrant purple-pink"),
        ("violet", "Purple"),
        ("magenta", "vec3f(1.0, 0.0, 1.0)"),
    ];

    colors
        .iter()
        .map(|(label, detail)| CompletionItem {
            label: label.to_string(),
            kind: Some(CompletionItemKind::COLOR),
            detail: Some(detail.to_string()),
            ..Default::default()
        })
        .collect()
}

/// Completion items for block keywords.
fn keyword_completions() -> Vec<CompletionItem> {
    let keywords: &[(&str, &str, &str)] = &[
        ("cinematic", "cinematic \"${1:Title}\" {\n\t$0\n}", "Top-level composition block"),
        ("layer", "layer \"${1:name}\" {\n\tfn: $0\n}", "Visual layer block"),
        ("arc", "arc {\n\t${1:0}s {\n\t\t$0\n\t}\n}", "Timeline animation block"),
        ("react", "react {\n\t$0\n}", "User interaction events"),
        ("resonate", "resonate {\n\t$0\n}", "Cross-layer modulation"),
        ("define", "define ${1:name} {\n\t$0\n}", "Reusable pattern definition"),
        ("lens", "lens {\n\t$0\n}", "Camera/render configuration"),
        ("import", "import \"${1:path}\"", "Import from another .game file"),
    ];

    keywords
        .iter()
        .map(|(label, snippet, detail)| CompletionItem {
            label: label.to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some(detail.to_string()),
            insert_text: Some(snippet.to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completion_list_includes_sdf_primitives() {
        let items = all_completions();
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(labels.contains(&"circle"), "missing circle");
        assert!(labels.contains(&"sphere"), "missing sphere");
        assert!(labels.contains(&"ring"), "missing ring");
        assert!(labels.contains(&"box"), "missing box");
        assert!(labels.contains(&"polygon"), "missing polygon");
        assert!(labels.contains(&"star"), "missing star");
        assert!(labels.contains(&"line"), "missing line");
    }

    #[test]
    fn completion_list_includes_block_keywords() {
        let items = all_completions();
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(labels.contains(&"cinematic"), "missing cinematic");
        assert!(labels.contains(&"layer"), "missing layer");
        assert!(labels.contains(&"arc"), "missing arc");
        assert!(labels.contains(&"define"), "missing define");
        assert!(labels.contains(&"import"), "missing import");
        assert!(labels.contains(&"lens"), "missing lens");
    }

    #[test]
    fn stage_completions_have_snippets() {
        let items = stage_completions();
        for item in &items {
            assert!(
                item.insert_text.is_some(),
                "stage '{}' missing insert_text",
                item.label
            );
            assert_eq!(
                item.insert_text_format,
                Some(InsertTextFormat::SNIPPET),
                "stage '{}' should use snippet format",
                item.label
            );
        }
    }
}
