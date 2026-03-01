//! Shader codegen orchestration.
//!
//! Generates WGSL and/or GLSL shaders from GAME AST, then hands off to
//! the runtime module to wrap them in Web Components or standalone HTML.

pub mod cast;
pub mod glsl;
pub mod memory;
pub mod stages;
pub mod wgsl;

use crate::ast::{Cinematic, Expr, LayerBody};
use crate::builtins;
use crate::error::CompileError;

/// Describes a user-defined uniform parameter extracted from layers.
#[derive(Debug, Clone)]
pub struct UniformInfo {
    pub name: String,
    pub default: f64,
}

/// Collected shader output for a single cinematic.
#[derive(Debug, Clone)]
pub struct ShaderOutput {
    pub name: String,
    pub wgsl_fragment: String,
    pub wgsl_vertex: String,
    pub glsl_fragment: String,
    pub glsl_vertex: String,
    pub uniforms: Vec<UniformInfo>,
    pub uses_memory: bool,
}

/// Extract user-defined uniform parameters from a cinematic's layers.
///
/// Any layer with `LayerBody::Params` contributes named uniforms.
/// Pipeline stages with `Ident` args that are NOT builtin names are also uniforms.
fn extract_uniforms(cinematic: &Cinematic) -> Vec<UniformInfo> {
    let mut uniforms = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for layer in &cinematic.layers {
        // Params-style layers declare uniforms directly
        if let LayerBody::Params(params) = &layer.body {
            for param in params {
                if seen.insert(param.name.clone()) {
                    let default = match &param.value {
                        Expr::Number(v) => *v,
                        _ => 0.0,
                    };
                    uniforms.push(UniformInfo {
                        name: param.name.clone(),
                        default,
                    });
                }
            }
        }

        // Pipeline stages: ident args that aren't builtins are user uniforms
        if let LayerBody::Pipeline(stages) = &layer.body {
            for stage in stages {
                for arg in &stage.args {
                    if let Expr::Ident(name) = &arg.value {
                        if builtins::lookup(name).is_none() && seen.insert(name.clone()) {
                            uniforms.push(UniformInfo {
                                name: name.clone(),
                                default: 0.0,
                            });
                        }
                    }
                }
            }
        }
    }

    uniforms
}

/// Validate all pipeline layers in a cinematic.
pub fn validate(cinematic: &Cinematic) -> Result<(), CompileError> {
    for layer in &cinematic.layers {
        if let LayerBody::Pipeline(pipeline) = &layer.body {
            stages::validate_pipeline(pipeline)?;
        }
    }
    // Cast type validation (checks pipeline output matches declared cast)
    cast::validate_casts(cinematic)?;
    Ok(())
}

/// Generate shaders for a single cinematic.
pub fn generate(cinematic: &Cinematic) -> Result<ShaderOutput, CompileError> {
    validate(cinematic)?;

    let uniforms = extract_uniforms(cinematic);

    let wgsl_fragment = wgsl::generate_fragment(cinematic, &uniforms);
    let glsl_fragment = glsl::generate_fragment(cinematic, &uniforms);

    let uses_memory = memory::any_layer_uses_memory(&cinematic.layers);

    Ok(ShaderOutput {
        name: cinematic.name.clone(),
        wgsl_fragment,
        wgsl_vertex: wgsl::vertex_shader().to_string(),
        glsl_fragment,
        glsl_vertex: glsl::vertex_shader().to_string(),
        uniforms,
        uses_memory,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::*;

    fn make_cinematic(stages: Vec<Stage>) -> Cinematic {
        Cinematic {
            name: "test".into(),
            layers: vec![Layer {
                name: "main".into(),
                opts: vec![],
                memory: None,
                cast: None,
                body: LayerBody::Pipeline(stages),
            }],
            arcs: vec![],
            resonates: vec![],
        }
    }

    #[test]
    fn generate_produces_both_shaders() {
        let cin = make_cinematic(vec![
            Stage { name: "circle".into(), args: vec![] },
            Stage { name: "glow".into(), args: vec![] },
        ]);
        let output = generate(&cin).unwrap();
        assert!(output.wgsl_fragment.contains("fn fs_main"));
        assert!(output.glsl_fragment.contains("void main()"));
        assert!(output.wgsl_vertex.contains("fn vs_main"));
        assert!(output.glsl_vertex.contains("#version 300 es"));
    }

    #[test]
    fn extract_ident_uniforms() {
        let cin = make_cinematic(vec![
            Stage {
                name: "circle".into(),
                args: vec![Arg {
                    name: None,
                    value: Expr::Ident("my_radius".into()),
                }],
            },
            Stage { name: "glow".into(), args: vec![] },
        ]);
        let uniforms = extract_uniforms(&cin);
        assert_eq!(uniforms.len(), 1);
        assert_eq!(uniforms[0].name, "my_radius");
    }

    #[test]
    fn validate_rejects_bad_pipeline() {
        let cin = make_cinematic(vec![
            Stage { name: "glow".into(), args: vec![] },
        ]);
        assert!(generate(&cin).is_err());
    }

    #[test]
    fn extract_param_uniforms() {
        let cin = Cinematic {
            name: "test".into(),
            layers: vec![Layer {
                name: "config".into(),
                opts: vec![],
                memory: None,
                cast: None,
                body: LayerBody::Params(vec![
                    Param {
                        name: "intensity".into(),
                        value: Expr::Number(0.5),
                        modulation: None,
                    },
                ]),
            }],
            arcs: vec![],
            resonates: vec![],
        };
        let uniforms = extract_uniforms(&cin);
        assert_eq!(uniforms.len(), 1);
        assert_eq!(uniforms[0].name, "intensity");
        assert!((uniforms[0].default - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn cast_validation_through_generate() {
        let cin = Cinematic {
            name: "test".into(),
            layers: vec![Layer {
                name: "main".into(),
                opts: vec![],
                memory: None,
                cast: Some("sdf".into()),
                body: LayerBody::Pipeline(vec![
                    Stage { name: "circle".into(), args: vec![] },
                ]),
            }],
            arcs: vec![],
            resonates: vec![],
        };
        assert!(generate(&cin).is_ok());
    }

    #[test]
    fn cast_mismatch_rejected_through_generate() {
        let cin = Cinematic {
            name: "test".into(),
            layers: vec![Layer {
                name: "main".into(),
                opts: vec![],
                memory: None,
                cast: Some("sdf".into()),
                body: LayerBody::Pipeline(vec![
                    Stage { name: "circle".into(), args: vec![] },
                    Stage { name: "glow".into(), args: vec![] },
                ]),
            }],
            arcs: vec![],
            resonates: vec![],
        };
        let err = generate(&cin).unwrap_err();
        assert!(err.to_string().contains("cast as 'sdf'"));
    }
}
