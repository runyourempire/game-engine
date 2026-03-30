//! Stage pipeline state machine for shader codegen.

use crate::ast::{Arg, Expr, Stage};
use crate::builtins::{self, ShaderState};
use crate::error::CompileError;

/// Resolve an argument value to a float string for shader emission.
pub fn resolve_arg(arg: &Arg, idx: usize, builtin_name: &str) -> String {
    match &arg.value {
        Expr::Number(v) => format!("{v:.6}"),
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
    let mut state = ShaderState::Position;
    let mut prev_stage_name: Option<&str> = None;

    for stage in stages {
        let builtin = builtins::lookup(&stage.name).ok_or_else(|| {
            let msg = format!("unknown stage function: '{}'", stage.name);
            let mut err = CompileError::validation(msg);
            if let Some(suggestion) = builtins::suggest(&stage.name) {
                err = err.with_help(format!("did you mean '{suggestion}'?"));
            }
            err
        })?;

        if builtin.input != state && builtin.input != ShaderState::Position {
            // Allow implicit state reset for Position-input stages (e.g., fbm | displace | circle).
            // Non-Position stages must match the current pipeline state exactly.
            let prev = prev_stage_name.unwrap_or("(start)");
            return Err(CompileError::validation(format!(
                "type mismatch: '{}' expects {} input, but pipeline is in {} state\n  \
                 help: the pipeline flows Position -> Sdf -> Color. \
                 '{}' produces {} output.",
                stage.name, builtin.input, state, prev, state
            )));
        }

        prev_stage_name = Some(&stage.name);
        state = builtin.output;
    }

    Ok(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Arg, Expr, Stage};

    fn stage(name: &str) -> Stage {
        Stage { name: name.into(), args: vec![] }
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
    fn unknown_stage_suggests_correction() {
        let stages = vec![stage("cicle")];
        let result = validate_pipeline(&stages);
        let err = result.unwrap_err();
        let help = err.help().expect("should have help text");
        assert!(help.contains("circle"), "help should suggest 'circle', got: {help}");
    }

    #[test]
    fn unknown_stage_no_suggestion_for_gibberish() {
        let stages = vec![stage("xyzxyzxyz")];
        let result = validate_pipeline(&stages);
        let err = result.unwrap_err();
        assert!(err.help().is_none(), "gibberish should not get a suggestion");
    }

    #[test]
    fn type_mismatch_explains_pipeline_flow() {
        // glow expects Sdf input, but pipeline starts in Position state
        let stages = vec![stage("glow")];
        let result = validate_pipeline(&stages);
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("type mismatch"), "should mention type mismatch: {err_msg}");
        assert!(err_msg.contains("Position -> Sdf -> Color"), "should explain flow: {err_msg}");
    }

    #[test]
    fn type_mismatch_after_color_stage() {
        // circle -> glow -> tint puts pipeline in Color state.
        // Then 'glow' expects Sdf — should error with previous stage info.
        let stages = vec![stage("circle"), stage("glow"), stage("tint"), stage("glow")];
        let result = validate_pipeline(&stages);
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("type mismatch"), "should mention type mismatch: {err_msg}");
        assert!(err_msg.contains("'tint'"), "should mention previous stage 'tint': {err_msg}");
    }
}
