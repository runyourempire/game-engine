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

    for stage in stages {
        let builtin = builtins::lookup(&stage.name).ok_or_else(|| {
            CompileError::validation(format!("unknown stage function: '{}'", stage.name))
        })?;

        if builtin.input != state {
            return Err(CompileError::validation(format!(
                "stage '{}' expects {:?} input but pipeline is in {:?} state",
                stage.name, builtin.input, state
            )));
        }

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
}
