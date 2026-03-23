//! Cast type validation — ensures pipeline output matches declared cast type.
//!
//! When a layer declares `cast sdf`, the pipeline must end in Sdf state.
//! When a layer declares `cast color`, the pipeline must end in Color state.

use crate::ast::{Cinematic, Layer, LayerBody};
use crate::builtins::ShaderState;
use crate::codegen::stages;
use crate::error::CompileError;

/// Map a cast type string to the expected shader state.
fn cast_to_state(cast: &str) -> Result<ShaderState, CompileError> {
    match cast {
        "sdf" | "distance" => Ok(ShaderState::Sdf),
        "color" | "rgba" => Ok(ShaderState::Color),
        "position" | "uv" => Ok(ShaderState::Position),
        other => Err(CompileError::validation(format!(
            "unknown cast type '{other}'. Expected: sdf, color, position"
        ))),
    }
}

/// Validate that a layer's pipeline output matches its declared cast type.
pub fn validate_layer_cast(layer: &Layer) -> Result<(), CompileError> {
    let cast_str = match &layer.cast {
        Some(c) => c,
        None => return Ok(()), // No cast declaration — anything goes
    };

    let expected = cast_to_state(cast_str)?;

    let pipeline = match &layer.body {
        LayerBody::Pipeline(stages) => stages,
        LayerBody::Params(_) | LayerBody::Conditional { .. } => {
            // Params-style and conditional layers — skip cast validation here
            return Ok(());
        }
    };

    if pipeline.is_empty() {
        return Err(CompileError::validation(format!(
            "layer '{}' is cast as '{cast_str}' but has an empty pipeline",
            layer.name
        )));
    }

    let actual = stages::validate_pipeline(pipeline)?;

    if actual != expected {
        return Err(CompileError::validation(format!(
            "layer '{}' is cast as '{cast_str}' ({expected:?}) but pipeline produces {actual:?}",
            layer.name
        )));
    }

    Ok(())
}

/// Validate all layers in a cinematic that have cast declarations.
pub fn validate_casts(cinematic: &Cinematic) -> Result<(), CompileError> {
    for layer in &cinematic.layers {
        validate_layer_cast(layer)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::*;

    fn layer_with_cast(name: &str, cast: &str, stages: Vec<Stage>) -> Layer {
        Layer {
            name: name.into(),
            opts: vec![],
            memory: None,
            opacity: None,
            cast: Some(cast.into()),
            blend: BlendMode::Add,
            feedback: false,
            body: LayerBody::Pipeline(stages),
        }
    }

    fn stage(name: &str) -> Stage {
        Stage {
            name: name.into(),
            args: vec![],
        }
    }

    #[test]
    fn cast_sdf_with_sdf_pipeline() {
        let layer = layer_with_cast("main", "sdf", vec![stage("circle")]);
        assert!(validate_layer_cast(&layer).is_ok());
    }

    #[test]
    fn cast_color_with_color_pipeline() {
        let layer = layer_with_cast(
            "main",
            "color",
            vec![stage("circle"), stage("glow"), stage("tint")],
        );
        assert!(validate_layer_cast(&layer).is_ok());
    }

    #[test]
    fn cast_sdf_rejects_color_pipeline() {
        let layer = layer_with_cast("main", "sdf", vec![stage("circle"), stage("glow")]);
        let err = validate_layer_cast(&layer).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("cast as 'sdf'"), "error: {msg}");
        assert!(msg.contains("Color"), "error: {msg}");
    }

    #[test]
    fn cast_color_rejects_sdf_pipeline() {
        let layer = layer_with_cast("main", "color", vec![stage("circle")]);
        let err = validate_layer_cast(&layer).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("cast as 'color'"), "error: {msg}");
        assert!(msg.contains("Sdf"), "error: {msg}");
    }

    #[test]
    fn unknown_cast_type_rejected() {
        let layer = layer_with_cast("main", "banana", vec![stage("circle")]);
        let err = validate_layer_cast(&layer).unwrap_err();
        assert!(err.to_string().contains("unknown cast type 'banana'"));
    }

    #[test]
    fn no_cast_always_passes() {
        let layer = Layer {
            name: "main".into(),
            opts: vec![],
            memory: None,
            opacity: None,
            cast: None,
            blend: BlendMode::Add,
            feedback: false,
            body: LayerBody::Pipeline(vec![stage("circle"), stage("glow")]),
        };
        assert!(validate_layer_cast(&layer).is_ok());
    }

    #[test]
    fn cast_position_with_transforms() {
        let layer = layer_with_cast(
            "prep",
            "position",
            vec![stage("translate"), stage("rotate"), stage("scale")],
        );
        assert!(validate_layer_cast(&layer).is_ok());
    }

    #[test]
    fn empty_pipeline_with_cast_rejected() {
        let layer = layer_with_cast("empty", "color", vec![]);
        let err = validate_layer_cast(&layer).unwrap_err();
        assert!(err.to_string().contains("empty pipeline"));
    }

    #[test]
    fn validate_casts_checks_all_layers() {
        let cin = Cinematic {
            name: "test".into(),
            layers: vec![
                layer_with_cast("a", "sdf", vec![stage("circle")]),
                layer_with_cast("b", "color", vec![stage("circle")]), // wrong!
            ],
            arcs: vec![],
            resonates: vec![],
            listen: None,
            voice: None,
            score: None,
            gravity: None,
            react: None,
            swarm: None,
            flow: None,
            particles: None,
            passes: vec![],
            cinematic_uses: vec![],
            matrix_coupling: None,
            matrix_color: None,
            props: None,
            dom: None,
            events: vec![],
            role: None,
            scene3d: None,
            textures: vec![],
        };
        let err = validate_casts(&cin).unwrap_err();
        assert!(err.to_string().contains("layer 'b'"));
    }

    #[test]
    fn distance_alias_works() {
        let layer = layer_with_cast("main", "distance", vec![stage("circle")]);
        assert!(validate_layer_cast(&layer).is_ok());
    }

    #[test]
    fn rgba_alias_works() {
        let layer = layer_with_cast("main", "rgba", vec![stage("circle"), stage("glow")]);
        assert!(validate_layer_cast(&layer).is_ok());
    }
}
