//! Resonance compilation — cross-layer parameter modulation.
//!
//! The `resonate {}` block specifies how layer params modulate each other at
//! runtime via JS. Example:
//!   resonate {
//!     fire.freq ~ ice.clarity * 2.0
//!     ice.density ~ fire.intensity * -1.5
//!     damping: 0.96
//!   }
//!
//! Audio band names (bass, mid, treble, energy, beat) can be used as source
//! expressions to create audio-reactive resonance:
//!   resonate {
//!     scale ~ bass * 0.5
//!     rotation ~ treble * 2.0
//!     damping: 0.96
//!   }
//!
//! This compiles to a JS function that runs each frame, updating param base
//! values based on other param values. The damping factor prevents runaway
//! feedback loops.

use std::collections::HashMap;

use crate::ast::ResonanceBlock;
use crate::codegen::expr::compile_expr_js;
use crate::codegen::CompiledParam;

/// Audio frequency band names that map to JS runtime variables.
const AUDIO_BANDS: &[(&str, &str)] = &[
    ("bass", "audioBass"),
    ("mid", "audioMid"),
    ("treble", "audioTreble"),
    ("energy", "audioEnergy"),
    ("beat", "audioBeat"),
];

/// Compiled resonance output — JS code to inject into the runtime.
pub struct CompiledResonance {
    /// JS function body that updates param bases based on cross-references.
    pub js_code: String,
    /// Damping factor (0..1, applied each frame to resonance contributions).
    pub damping: f64,
}

/// Compile a ResonanceBlock into JS that can be injected into the runtime frame loop.
///
/// `params` is the list of compiled params with their indices, so we can map
/// `layer.param` references to uniform indices.
pub fn compile_resonance(
    block: &ResonanceBlock,
    params: &[CompiledParam],
) -> CompiledResonance {
    // Clamp damping to [0, 1] to prevent runaway feedback
    let raw_damping = block.damping.unwrap_or(0.95);
    let damping = raw_damping.clamp(0.0, 1.0);

    if block.bindings.is_empty() {
        return CompiledResonance {
            js_code: String::new(),
            damping,
        };
    }

    // Build a param name -> index map
    let param_map: HashMap<&str, usize> = params.iter()
        .enumerate()
        .map(|(i, p)| (p.name.as_str(), i))
        .collect();

    let mut js = String::with_capacity(512);
    js.push_str("// ── Resonance: cross-layer parameter modulation ──\n");
    js.push_str("(function resonanceUpdate(params, dt) {\n");

    // Guard for empty params array
    js.push_str("  if (!params || params.length === 0) return;\n");

    js.push_str(&format!("  const damp = {};\n", damping));

    // Track contributions separately to avoid order-dependent updates
    js.push_str("  const deltas = new Float64Array(params.length);\n");

    for binding in &block.bindings {
        // Parse target: "layer.param" or just "param"
        let target_name = extract_param_name(&binding.target);
        let target_idx = param_map.get(target_name.as_str());

        if let Some(&idx) = target_idx {
            // Compile the source expression to JS
            let source_js = compile_expr_js(&binding.source);

            // Replace param references and audio band names with their JS lookups
            let resolved_js = resolve_param_refs(&source_js, &param_map);

            js.push_str(&format!(
                "  deltas[{idx}] += ({resolved_js}) * damp * dt;\n"
            ));
        }
    }

    // Apply deltas to param bases
    js.push_str("  for (let i = 0; i < params.length; i++) {\n");
    js.push_str("    params[i].base += deltas[i];\n");
    js.push_str("  }\n");
    js.push_str("})(params, 1/60);\n");

    CompiledResonance {
        js_code: js,
        damping,
    }
}

/// Extract the param name from a dotted path like "fire.freq" -> "freq"
/// or a plain name like "freq" -> "freq".
fn extract_param_name(target: &str) -> String {
    if let Some(pos) = target.rfind('.') {
        target[pos + 1..].to_string()
    } else {
        target.to_string()
    }
}

/// Replace param names in a JS expression with `params[N].value` lookups,
/// and audio band names with their JS runtime variables.
fn resolve_param_refs(js: &str, param_map: &HashMap<&str, usize>) -> String {
    let mut result = js.to_string();

    // Sort by name length (longest first) to avoid partial replacements
    let mut entries: Vec<(&&str, &usize)> = param_map.iter().collect();
    entries.sort_by(|a, b| b.0.len().cmp(&a.0.len()));

    for (name, idx) in entries {
        // Only replace standalone identifiers (not inside other words)
        let replacement = format!("params[{idx}].value");
        result = replace_word(&result, name, &replacement);
    }

    // Resolve audio band names to their JS runtime variables.
    // This allows resonance bindings like `scale ~ bass * 0.5` to read
    // from the Web Audio analysis data.
    for &(band_name, js_var) in AUDIO_BANDS {
        result = replace_word(&result, band_name, js_var);
    }

    result
}

/// Replace a word (identifier) in a string, only matching whole words.
fn replace_word(text: &str, word: &str, replacement: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut i = 0;

    while i < text.len() {
        if text[i..].starts_with(word) {
            let before_ok = if i == 0 {
                true
            } else {
                let prev = text.as_bytes()[i - 1] as char;
                !prev.is_alphanumeric() && prev != '_'
            };
            let after_ok = if i + word.len() >= text.len() {
                true
            } else {
                let next = text.as_bytes()[i + word.len()] as char;
                !next.is_alphanumeric() && next != '_'
            };

            if before_ok && after_ok {
                result.push_str(replacement);
                i += word.len();
                continue;
            }
        }
        result.push(text.as_bytes()[i] as char);
        i += 1;
    }

    result
}
