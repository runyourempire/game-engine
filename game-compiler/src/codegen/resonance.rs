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
//! This compiles to a JS function that runs each frame, updating param base
//! values based on other param values. The damping factor prevents runaway
//! feedback loops.

use std::collections::{HashMap, HashSet};

use crate::ast::ResonanceBlock;
use crate::codegen::expr::compile_expr_js;
use crate::codegen::CompiledParam;

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
    warnings: &mut Vec<String>,
) -> CompiledResonance {
    let damping = block.damping.unwrap_or(0.95);

    if block.bindings.is_empty() {
        return CompiledResonance {
            js_code: String::new(),
            damping,
        };
    }

    // Build a param name → index map
    let param_map: HashMap<&str, usize> = params.iter()
        .enumerate()
        .map(|(i, p)| (p.name.as_str(), i))
        .collect();

    // ── Cycle detection ────────────────────────────────────────────────
    // Build dependency graph: target → set of source params it reads
    let mut deps: HashMap<String, HashSet<String>> = HashMap::new();
    for binding in &block.bindings {
        let target_name = extract_param_name(&binding.target);
        let source_js = compile_expr_js(&binding.source);
        let source_params: HashSet<String> = param_map.keys()
            .filter(|&&name| is_word_present(&source_js, name))
            .map(|&name| name.to_string())
            .collect();
        deps.entry(target_name).or_default().extend(source_params);
    }

    // Check for cycles: if A depends on B and B depends on A (mutual), warn without damping
    if damping >= 1.0 {
        if let Some(cycle) = detect_cycle(&deps) {
            warnings.push(format!(
                "resonance cycle detected ({}) without damping < 1.0 — this will cause runaway feedback. Add `damping: 0.95` or similar",
                cycle.join(" <-> ")
            ));
        }
    }

    // ── Generate JS ──────────────────────────────────────────────────
    let mut js = String::with_capacity(512);
    js.push_str("// ── Resonance: cross-layer parameter modulation ──\n");
    js.push_str("(function resonanceUpdate(params, dt) {\n");
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

            // Replace param references in source with params[N].value lookups
            let resolved_js = resolve_param_refs(&source_js, &param_map);

            js.push_str(&format!(
                "  deltas[{idx}] += ({resolved_js}) * damp * dt;\n"
            ));
        } else {
            warnings.push(format!(
                "resonance target '{}' does not match any declared param",
                binding.target
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

/// Extract the param name from a dotted path like "fire.freq" → "freq"
/// or a plain name like "freq" → "freq".
fn extract_param_name(target: &str) -> String {
    if let Some(pos) = target.rfind('.') {
        target[pos + 1..].to_string()
    } else {
        target.to_string()
    }
}

/// Check if a word (identifier) appears as a standalone token in the string.
fn is_word_present(text: &str, word: &str) -> bool {
    let mut start = 0;
    while let Some(pos) = text[start..].find(word) {
        let abs_pos = start + pos;
        let before_ok = abs_pos == 0 || {
            let prev = text.as_bytes()[abs_pos - 1];
            !prev.is_ascii_alphanumeric() && prev != b'_'
        };
        let after_pos = abs_pos + word.len();
        let after_ok = after_pos >= text.len() || {
            let next = text.as_bytes()[after_pos];
            !next.is_ascii_alphanumeric() && next != b'_'
        };
        if before_ok && after_ok {
            return true;
        }
        start = abs_pos + 1;
    }
    false
}

/// Detect cycles in the dependency graph. Returns the first cycle found.
fn detect_cycle(deps: &HashMap<String, HashSet<String>>) -> Option<Vec<String>> {
    // Simple cycle detection: check for mutual dependencies
    for (target, sources) in deps {
        for source in sources {
            if let Some(source_deps) = deps.get(source) {
                if source_deps.contains(target) {
                    return Some(vec![target.clone(), source.clone()]);
                }
            }
        }
    }
    None
}

/// Replace param names in a JS expression with `params[N].value` lookups.
fn resolve_param_refs(js: &str, param_map: &HashMap<&str, usize>) -> String {
    let mut result = js.to_string();

    // Sort by name length (longest first) to avoid partial replacements
    let mut entries: Vec<(&&str, &usize)> = param_map.iter().collect();
    entries.sort_by(|a, b| b.0.len().cmp(&a.0.len()));

    for (name, idx) in entries {
        let replacement = format!("params[{idx}].value");
        result = replace_word(&result, name, &replacement);
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
                let prev = text.as_bytes()[i - 1];
                !prev.is_ascii_alphanumeric() && prev != b'_'
            };
            let after_ok = if i + word.len() >= text.len() {
                true
            } else {
                let next = text.as_bytes()[i + word.len()];
                !next.is_ascii_alphanumeric() && next != b'_'
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
