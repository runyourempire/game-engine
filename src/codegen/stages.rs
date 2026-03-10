//! Stage pipeline state machine for shader codegen.

use crate::ast::{Arg, Expr, FnDef, Stage};

/// Auto-insert bridge stages (glow) when SDF→Color gap is detected.
/// Returns the augmented pipeline and any compiler notes.
pub fn auto_bridge_pipeline(stages: &[Stage], fns: &[FnDef]) -> (Vec<Stage>, Vec<String>) {
    let mut result = Vec::new();
    let mut notes = Vec::new();
    let mut state = ShaderState::Position;

    for stage in stages {
        if let Some(builtin) = builtins::lookup(&stage.name) {
            if state == ShaderState::Sdf && builtin.input == ShaderState::Color {
                // Auto-insert glow(1.5) as bridge
                result.push(Stage {
                    name: "glow".to_string(),
                    args: vec![Arg {
                        name: None,
                        value: Expr::Number(1.5),
                    }],
                });
                notes.push(format!(
                    "auto-inserted glow(1.5) before '{}' (SDF→Color bridge)",
                    stage.name
                ));
                state = ShaderState::Color;
            }
            if builtin.input == state {
                state = builtin.output;
            }
        } else if let Some(fn_def) = fns.iter().find(|f| f.name == stage.name) {
            let fn_input = fn_def
                .body
                .first()
                .and_then(|s| builtins::lookup(&s.name))
                .map(|b| b.input)
                .unwrap_or(ShaderState::Position);
            if state == ShaderState::Sdf && fn_input == ShaderState::Color {
                result.push(Stage {
                    name: "glow".to_string(),
                    args: vec![Arg {
                        name: None,
                        value: Expr::Number(1.5),
                    }],
                });
                notes.push(format!(
                    "auto-inserted glow(1.5) before '{}' (SDF→Color bridge)",
                    stage.name
                ));
                state = ShaderState::Color;
            }
            if let Ok(fn_output) = validate_pipeline_with_fns(&fn_def.body, fns) {
                state = fn_output;
            }
        }
        result.push(stage.clone());
    }

    // Auto-bridge at end if pipeline ends in Sdf state
    if state == ShaderState::Sdf {
        result.push(Stage {
            name: "glow".to_string(),
            args: vec![Arg {
                name: None,
                value: Expr::Number(1.5),
            }],
        });
        notes.push("auto-inserted glow(1.5) at end of pipeline (SDF→Color bridge)".to_string());
    }

    (result, notes)
}
use crate::builtins::{self, ShaderState};
use crate::error::CompileError;

/// Find the closest matching builtin or user fn name for "did you mean?" suggestions.
fn suggest_name(unknown: &str, fns: &[FnDef]) -> Option<String> {
    let builtin_names: Vec<String> = builtins::all_names().map(|s| s.to_string()).collect();
    let fn_names: Vec<String> = fns.iter().map(|f| f.name.clone()).collect();

    let mut best: Option<(String, usize)> = None;
    for name in builtin_names.iter().chain(fn_names.iter()) {
        let d = edit_distance(unknown, name);
        // Only suggest if edit distance is small relative to name length
        if d <= 2 || (d <= 3 && unknown.len() > 5) {
            if best.is_none() || d < best.as_ref().unwrap().1 {
                best = Some((name.clone(), d));
            }
        }
    }
    best.map(|(name, _)| name)
}

/// Simple Levenshtein edit distance.
fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let (m, n) = (a.len(), b.len());
    let mut dp = vec![vec![0usize; n + 1]; m + 1];
    for i in 0..=m {
        dp[i][0] = i;
    }
    for j in 0..=n {
        dp[0][j] = j;
    }
    for i in 1..=m {
        for j in 1..=n {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            dp[i][j] = (dp[i - 1][j] + 1)
                .min(dp[i][j - 1] + 1)
                .min(dp[i - 1][j - 1] + cost);
        }
    }
    dp[m][n]
}

/// Suggest the missing Sdf→Color bridge when a color stage follows an SDF generator.
fn suggest_bridge(stage_name: &str, state: ShaderState) -> Option<String> {
    if state == ShaderState::Sdf {
        if let Some(builtin) = builtins::lookup(stage_name) {
            if builtin.input == ShaderState::Color {
                return Some(format!(
                    "stage '{}' expects Color input but pipeline is in Sdf state. \
                     Did you mean to add 'glow()' or 'shade()' before '{}'?",
                    stage_name, stage_name
                ));
            }
        }
    }
    None
}

/// Resolve an argument value to a float string for shader emission.
pub fn resolve_arg(arg: &Arg, idx: usize, builtin_name: &str) -> String {
    match &arg.value {
        Expr::Number(v) => format!("{v:.6}"),
        Expr::Color(r, g, b) => {
            // When a hex color is used as a single arg, return component by idx
            match idx % 3 {
                0 => format!("{r:.6}"),
                1 => format!("{g:.6}"),
                _ => format!("{b:.6}"),
            }
        }
        Expr::Ident(name) => name.clone(),
        Expr::DottedIdent { object, field } => format!("{object}_{field}"),
        _ => {
            // Fallback: use builtin default if available
            builtins::lookup(builtin_name)
                .and_then(|b| b.params.get(idx))
                .and_then(|p| p.default)
                .map(|d| format!("{d:.6}"))
                .unwrap_or_else(|| "0.0".into())
        }
    }
}

/// Get an arg value by name or position.
pub fn get_arg(args: &[Arg], name: &str, pos: usize, stage_name: &str) -> String {
    // Try named first
    for arg in args {
        if arg.name.as_deref() == Some(name) {
            return resolve_arg(arg, pos, stage_name);
        }
    }
    // Hex color expansion: tint(#RRGGBB) distributes r/g/b across pos 0/1/2
    if pos > 0 && !args.is_empty() {
        if let Expr::Color(r, g, b) = &args[0].value {
            return match pos {
                0 => format!("{r:.6}"),
                1 => format!("{g:.6}"),
                2 => format!("{b:.6}"),
                _ => "0.0".to_string(),
            };
        }
    }
    // Try positional
    if let Some(arg) = args.get(pos) {
        return resolve_arg(arg, pos, stage_name);
    }
    // Fallback to default
    builtins::lookup(stage_name)
        .and_then(|b| b.params.get(pos))
        .and_then(|p| p.default)
        .map(|d| format!("{d:.6}"))
        .unwrap_or_else(|| "0.0".into())
}

/// Validate a pipeline of stages — returns error if state transitions are invalid.
pub fn validate_pipeline(stages: &[Stage]) -> Result<ShaderState, CompileError> {
    validate_pipeline_with_fns(stages, &[])
}

/// Validate a pipeline with user-defined function awareness.
pub fn validate_pipeline_with_fns(
    stages: &[Stage],
    fns: &[FnDef],
) -> Result<ShaderState, CompileError> {
    let mut state = ShaderState::Position;

    for stage in stages {
        // Try builtin first
        if let Some(builtin) = builtins::lookup(&stage.name) {
            if builtin.input != state {
                // Check for common Sdf→Color bridge mistake
                if let Some(hint) = suggest_bridge(&stage.name, state) {
                    return Err(CompileError::validation(hint));
                }
                return Err(CompileError::validation(format!(
                    "stage '{}' expects {:?} input but pipeline is in {:?} state",
                    stage.name, builtin.input, state
                )));
            }
            state = builtin.output;
        }
        // Try user-defined fn
        else if let Some(fn_def) = fns.iter().find(|f| f.name == stage.name) {
            let fn_output = validate_pipeline_with_fns(&fn_def.body, fns)?;
            let fn_input = fn_def
                .body
                .first()
                .and_then(|s| builtins::lookup(&s.name))
                .map(|b| b.input)
                .unwrap_or(ShaderState::Position);

            if fn_input != state {
                return Err(CompileError::validation(format!(
                    "function '{}' expects {:?} input but pipeline is in {:?} state",
                    stage.name, fn_input, state
                )));
            }
            state = fn_output;
        } else {
            let mut msg = format!("unknown stage function: '{}'", stage.name);
            if let Some(suggestion) = suggest_name(&stage.name, fns) {
                msg.push_str(&format!(". Did you mean '{suggestion}'?"));
            }
            // Add valid alternatives for current state
            let valid = builtins::valid_next_stages(state);
            if !valid.is_empty() {
                let shown: Vec<&str> = valid.iter().take(8).copied().collect();
                msg.push_str(&format!(
                    "\n  Valid stages for {:?} state: {}",
                    state,
                    shown.join(", ")
                ));
                if valid.len() > 8 {
                    msg.push_str(&format!(" (and {} more)", valid.len() - 8));
                }
            }
            return Err(CompileError::validation(msg));
        }
    }

    Ok(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Arg, Expr, Stage};

    fn stage(name: &str) -> Stage {
        Stage {
            name: name.into(),
            args: vec![],
        }
    }

    #[test]
    fn valid_pipeline_circle_glow_tint() {
        let stages = vec![stage("circle"), stage("glow"), stage("tint")];
        let result = validate_pipeline(&stages);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ShaderState::Color);
    }

    #[test]
    fn invalid_pipeline_glow_from_position() {
        let stages = vec![stage("glow")];
        let result = validate_pipeline(&stages);
        assert!(result.is_err());
    }

    #[test]
    fn transforms_stay_in_position() {
        let stages = vec![stage("translate"), stage("rotate"), stage("circle")];
        let result = validate_pipeline(&stages);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ShaderState::Sdf);
    }

    #[test]
    fn get_arg_with_default() {
        let args: Vec<Arg> = vec![];
        let val = get_arg(&args, "radius", 0, "circle");
        assert_eq!(val, "0.200000");
    }

    #[test]
    fn get_arg_positional() {
        let args = vec![Arg {
            name: None,
            value: Expr::Number(0.5),
        }];
        let val = get_arg(&args, "radius", 0, "circle");
        assert_eq!(val, "0.500000");
    }

    #[test]
    fn get_arg_named() {
        let args = vec![Arg {
            name: Some("radius".into()),
            value: Expr::Number(0.75),
        }];
        let val = get_arg(&args, "radius", 0, "circle");
        assert_eq!(val, "0.750000");
    }

    #[test]
    fn phase7_warp_voronoi_palette_pipeline() {
        let stages = vec![stage("warp"), stage("voronoi"), stage("palette")];
        let result = validate_pipeline(&stages);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ShaderState::Color);
    }

    #[test]
    fn phase7_distort_radial_fade_glow() {
        let stages = vec![stage("distort"), stage("radial_fade"), stage("glow")];
        let result = validate_pipeline(&stages);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ShaderState::Color);
    }

    #[test]
    fn phase7_polar_simplex_shade() {
        let stages = vec![stage("polar"), stage("simplex"), stage("shade")];
        let result = validate_pipeline(&stages);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ShaderState::Color);
    }

    #[test]
    fn boolean_union_pipeline() {
        let stages = vec![stage("union"), stage("glow"), stage("tint")];
        let result = validate_pipeline(&stages);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ShaderState::Color);
    }

    #[test]
    fn smooth_union_pipeline() {
        let stages = vec![stage("smooth_union"), stage("glow")];
        let result = validate_pipeline(&stages);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ShaderState::Color);
    }

    #[test]
    fn spatial_repeat_circle_pipeline() {
        let stages = vec![stage("repeat"), stage("circle"), stage("glow")];
        let result = validate_pipeline(&stages);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ShaderState::Color);
    }

    #[test]
    fn mirror_radial_pipeline() {
        let stages = vec![
            stage("mirror"),
            stage("radial"),
            stage("circle"),
            stage("glow"),
        ];
        let result = validate_pipeline(&stages);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ShaderState::Color);
    }

    #[test]
    fn shape_modifier_round_pipeline() {
        let stages = vec![stage("box"), stage("round"), stage("glow")];
        let result = validate_pipeline(&stages);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ShaderState::Color);
    }

    #[test]
    fn shape_modifier_shell_pipeline() {
        let stages = vec![stage("circle"), stage("shell"), stage("glow")];
        let result = validate_pipeline(&stages);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ShaderState::Color);
    }

    #[test]
    fn shape_modifier_onion_pipeline() {
        let stages = vec![stage("circle"), stage("onion"), stage("glow")];
        let result = validate_pipeline(&stages);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ShaderState::Color);
    }

    #[test]
    fn new_primitives_pipeline() {
        for prim in [
            "line", "capsule", "triangle", "arc_sdf", "cross", "heart", "egg", "spiral", "grid",
        ] {
            let stages = vec![stage(prim), stage("glow")];
            let result = validate_pipeline(&stages);
            assert!(result.is_ok(), "primitive '{prim}' should produce Sdf");
            assert_eq!(result.unwrap(), ShaderState::Color);
        }
    }

    #[test]
    fn outline_is_color_to_color() {
        let stages = vec![stage("circle"), stage("glow"), stage("outline")];
        let result = validate_pipeline(&stages);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ShaderState::Color);
    }

    // v0.4 — user-defined fn validation

    #[test]
    fn fn_pipeline_validates() {
        let fns = vec![FnDef {
            name: "dot".into(),
            params: vec![],
            body: vec![stage("circle"), stage("glow"), stage("tint")],
        }];
        let stages = vec![stage("dot")];
        let result = validate_pipeline_with_fns(&stages, &fns);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ShaderState::Color);
    }

    #[test]
    fn fn_wrong_input_state_rejected() {
        // fn that expects Position but pipeline already in Sdf state
        let fns = vec![FnDef {
            name: "post".into(),
            params: vec![],
            body: vec![stage("circle"), stage("glow")],
        }];
        // circle produces Sdf, then "post" expects Position input
        let stages = vec![stage("circle"), stage("post")];
        let result = validate_pipeline_with_fns(&stages, &fns);
        assert!(result.is_err());
    }

    #[test]
    fn fn_chained_with_builtins() {
        // fn sdf_shape() { circle() } → pipeline: sdf_shape | glow | tint
        let fns = vec![FnDef {
            name: "sdf_shape".into(),
            params: vec![],
            body: vec![stage("circle")],
        }];
        let stages = vec![stage("sdf_shape"), stage("glow"), stage("tint")];
        let result = validate_pipeline_with_fns(&stages, &fns);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ShaderState::Color);
    }

    #[test]
    fn unknown_fn_rejected() {
        let stages = vec![stage("nonexistent_fn")];
        let result = validate_pipeline_with_fns(&stages, &[]);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("unknown stage function"));
    }

    #[test]
    fn morph_validates_as_sdf() {
        let stages = vec![stage("morph"), stage("glow")];
        let result = validate_pipeline(&stages);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ShaderState::Color);
    }

    // ── Error message tests ─────────────────────────────────

    #[test]
    fn unknown_stage_suggests_typo_fix() {
        let stages = vec![stage("circl")]; // typo for "circle"
        let result = validate_pipeline(&stages);
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Did you mean 'circle'?"), "got: {err}");
    }

    #[test]
    fn unknown_stage_suggests_close_match() {
        let stages = vec![stage("glo")]; // typo for "glow"
        let result = validate_pipeline(&stages);
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Did you mean 'glow'?"), "got: {err}");
    }

    #[test]
    fn unknown_stage_suggests_user_fn() {
        let fns = vec![FnDef {
            name: "my_shape".into(),
            params: vec![],
            body: vec![stage("circle"), stage("glow")],
        }];
        let stages = vec![stage("my_shap")]; // typo for user fn
        let result = validate_pipeline_with_fns(&stages, &fns);
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Did you mean 'my_shape'?"), "got: {err}");
    }

    #[test]
    fn bridge_hint_when_tint_follows_sdf() {
        // circle() produces Sdf, tint() expects Color
        let stages = vec![stage("circle"), stage("tint")];
        let result = validate_pipeline(&stages);
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("glow") || err.contains("shade"),
            "should suggest bridge stage, got: {err}"
        );
    }

    #[test]
    fn no_suggestion_for_very_different_name() {
        let stages = vec![stage("zzzzzzzzz")];
        let result = validate_pipeline(&stages);
        let err = result.unwrap_err().to_string();
        assert!(err.contains("unknown stage function"));
        assert!(
            !err.contains("Did you mean"),
            "should not suggest for very different name: {err}"
        );
    }

    // ── Auto-bridge tests ─────────────────────────────────

    #[test]
    fn auto_bridge_inserts_glow() {
        let stages = vec![stage("circle"), stage("tint")];
        let (bridged, notes) = auto_bridge_pipeline(&stages, &[]);
        assert_eq!(bridged.len(), 3);
        assert_eq!(bridged[1].name, "glow");
        assert!(!notes.is_empty());
        let result = validate_pipeline(&bridged);
        assert!(result.is_ok());
    }

    #[test]
    fn auto_bridge_end_of_pipeline() {
        let stages = vec![stage("circle")];
        let (bridged, notes) = auto_bridge_pipeline(&stages, &[]);
        assert_eq!(bridged.len(), 2);
        assert_eq!(bridged[1].name, "glow");
        assert!(!notes.is_empty());
    }

    #[test]
    fn auto_bridge_not_needed() {
        let stages = vec![stage("circle"), stage("glow"), stage("tint")];
        let (bridged, notes) = auto_bridge_pipeline(&stages, &[]);
        assert_eq!(bridged.len(), 3);
        assert!(notes.is_empty());
    }

    #[test]
    fn auto_bridge_with_transforms() {
        let stages = vec![stage("warp"), stage("circle"), stage("bloom")];
        let (bridged, _notes) = auto_bridge_pipeline(&stages, &[]);
        assert_eq!(bridged.len(), 4);
        assert_eq!(bridged[2].name, "glow");
    }

    #[test]
    fn edit_distance_identical() {
        assert_eq!(edit_distance("circle", "circle"), 0);
    }

    #[test]
    fn edit_distance_one_char() {
        assert_eq!(edit_distance("glow", "glo"), 1);
        assert_eq!(edit_distance("tint", "tit"), 1);
    }

    #[test]
    fn edit_distance_two_chars() {
        assert_eq!(edit_distance("circle", "circl"), 1);
        assert_eq!(edit_distance("voronoi", "voronei"), 1);
    }
}
