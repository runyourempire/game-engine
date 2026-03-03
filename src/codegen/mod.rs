//! Shader codegen orchestration.
//!
//! Generates WGSL and/or GLSL shaders from GAME AST, then hands off to
//! the runtime module to wrap them in Web Components or standalone HTML.

pub mod arc;
pub mod breed;
pub mod cast;
pub mod glsl;
pub mod gravity;
pub mod listen;
pub mod memory;
pub mod project;
pub mod resonate;
pub mod score;
pub mod stages;
pub mod temporal;
pub mod voice;
pub mod wgsl;

use crate::ast::{Cinematic, Expr, LayerBody, Param};
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
    /// Collected JS classes (listen, voice, score, breed, temporal, gravity).
    pub js_modules: Vec<String>,
    /// Gravity compute shader (separate pipeline).
    pub compute_wgsl: Option<String>,
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

/// Collect all `Param` references from a cinematic's `LayerBody::Params` layers.
fn collect_all_params(cinematic: &Cinematic) -> Vec<&Param> {
    cinematic
        .layers
        .iter()
        .filter_map(|layer| {
            if let LayerBody::Params(params) = &layer.body {
                Some(params.iter())
            } else {
                None
            }
        })
        .flatten()
        .collect()
}

/// Generate shaders for a single cinematic.
pub fn generate(cinematic: &Cinematic) -> Result<ShaderOutput, CompileError> {
    validate(cinematic)?;

    let uniforms = extract_uniforms(cinematic);

    let wgsl_fragment = wgsl::generate_fragment(cinematic, &uniforms);
    let glsl_fragment = glsl::generate_fragment(cinematic, &uniforms);

    let uses_memory = memory::any_layer_uses_memory(&cinematic.layers);

    // Collect JS feature modules
    let mut js_modules = Vec::new();

    // Temporal: collect params from all layers
    let all_params: Vec<Param> = collect_all_params(cinematic).into_iter().cloned().collect();
    if temporal::any_param_uses_temporal(&all_params) {
        let (init, update) = temporal::generate_temporal_js(&all_params);
        js_modules.push(format!("{init}\n{update}"));
    }

    // Listen → GameListenPipeline class
    if let Some(ref lb) = cinematic.listen {
        js_modules.push(listen::generate_listen_js(lb));
    }

    // Voice → GameVoiceSynth class
    if let Some(ref vb) = cinematic.voice {
        js_modules.push(voice::generate_voice_js(vb));
    }

    // Score → GameScorePlayer class
    if let Some(ref sb) = cinematic.score {
        js_modules.push(score::generate_score_js(sb));
    }

    // Gravity → compute WGSL + GameGravitySim JS class
    let compute_wgsl = if let Some(ref gb) = cinematic.gravity {
        let n = 1024u32;
        js_modules.push(gravity::generate_compute_runtime_js(n));
        Some(gravity::generate_compute_wgsl(gb, n))
    } else {
        None
    };

    // Resonate → GameResonanceNetwork JS class (parametric coupling)
    if !cinematic.resonates.is_empty() {
        let resonate_js = resonate::generate_resonate_js(&cinematic.resonates);
        if !resonate_js.is_empty() {
            js_modules.push(resonate_js);
        }
    }

    // Arc → GameArcTimeline JS class (parameter animation)
    if !cinematic.arcs.is_empty() {
        let arc_js = arc::generate_arc_js(&cinematic.arcs);
        if !arc_js.is_empty() {
            js_modules.push(arc_js);
        }
    }

    Ok(ShaderOutput {
        name: cinematic.name.clone(),
        wgsl_fragment,
        wgsl_vertex: wgsl::vertex_shader().to_string(),
        glsl_fragment,
        glsl_vertex: glsl::vertex_shader().to_string(),
        uniforms,
        uses_memory,
        js_modules,
        compute_wgsl,
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
            listen: None,
            voice: None,
            score: None,
            gravity: None,
        }
    }

    #[test]
    fn generate_produces_both_shaders() {
        let cin = make_cinematic(vec![
            Stage {
                name: "circle".into(),
                args: vec![],
            },
            Stage {
                name: "glow".into(),
                args: vec![],
            },
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
            Stage {
                name: "glow".into(),
                args: vec![],
            },
        ]);
        let uniforms = extract_uniforms(&cin);
        assert_eq!(uniforms.len(), 1);
        assert_eq!(uniforms[0].name, "my_radius");
    }

    #[test]
    fn validate_rejects_bad_pipeline() {
        let cin = make_cinematic(vec![Stage {
            name: "glow".into(),
            args: vec![],
        }]);
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
                body: LayerBody::Params(vec![Param {
                    name: "intensity".into(),
                    value: Expr::Number(0.5),
                    modulation: None,
                    temporal_ops: vec![],
                }]),
            }],
            arcs: vec![],
            resonates: vec![],
            listen: None,
            voice: None,
            score: None,
            gravity: None,
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
                body: LayerBody::Pipeline(vec![Stage {
                    name: "circle".into(),
                    args: vec![],
                }]),
            }],
            arcs: vec![],
            resonates: vec![],
            listen: None,
            voice: None,
            score: None,
            gravity: None,
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
                    Stage {
                        name: "circle".into(),
                        args: vec![],
                    },
                    Stage {
                        name: "glow".into(),
                        args: vec![],
                    },
                ]),
            }],
            arcs: vec![],
            resonates: vec![],
            listen: None,
            voice: None,
            score: None,
            gravity: None,
        };
        let err = generate(&cin).unwrap_err();
        assert!(err.to_string().contains("cast as 'sdf'"));
    }

    #[test]
    fn generate_with_listen_produces_js_module() {
        let cin = Cinematic {
            name: "audio-viz".into(),
            layers: vec![Layer {
                name: "main".into(),
                opts: vec![],
                memory: None,
                cast: None,
                body: LayerBody::Pipeline(vec![
                    Stage {
                        name: "circle".into(),
                        args: vec![],
                    },
                    Stage {
                        name: "glow".into(),
                        args: vec![],
                    },
                ]),
            }],
            arcs: vec![],
            resonates: vec![],
            listen: Some(crate::ast::ListenBlock {
                signals: vec![crate::ast::ListenSignal {
                    name: "onset".into(),
                    algorithm: "attack".into(),
                    params: vec![],
                }],
            }),
            voice: None,
            score: None,
            gravity: None,
        };
        let output = generate(&cin).unwrap();
        assert_eq!(output.js_modules.len(), 1);
        assert!(output.js_modules[0].contains("GameListenPipeline"));
        assert!(output.compute_wgsl.is_none());
    }

    #[test]
    fn generate_with_gravity_produces_compute() {
        let cin = Cinematic {
            name: "particles".into(),
            layers: vec![Layer {
                name: "main".into(),
                opts: vec![],
                memory: None,
                cast: None,
                body: LayerBody::Pipeline(vec![
                    Stage {
                        name: "circle".into(),
                        args: vec![],
                    },
                    Stage {
                        name: "glow".into(),
                        args: vec![],
                    },
                ]),
            }],
            arcs: vec![],
            resonates: vec![],
            listen: None,
            voice: None,
            score: None,
            gravity: Some(crate::ast::GravityBlock {
                force_law: Expr::Number(1.0),
                damping: 0.99,
                bounds: crate::ast::BoundsMode::Reflect,
            }),
        };
        let output = generate(&cin).unwrap();
        assert!(output.compute_wgsl.is_some());
        assert!(output.compute_wgsl.unwrap().contains("cs_main"));
        assert!(output
            .js_modules
            .iter()
            .any(|m| m.contains("GameGravitySim")));
    }

    #[test]
    fn generate_default_has_empty_js_modules() {
        let cin = make_cinematic(vec![
            Stage {
                name: "circle".into(),
                args: vec![],
            },
            Stage {
                name: "glow".into(),
                args: vec![],
            },
        ]);
        let output = generate(&cin).unwrap();
        assert!(output.js_modules.is_empty());
        assert!(output.compute_wgsl.is_none());
    }

    #[test]
    fn generate_with_resonate_produces_js_module() {
        let cin = Cinematic {
            name: "coupled".into(),
            layers: vec![Layer {
                name: "main".into(),
                opts: vec![],
                memory: None,
                cast: None,
                body: LayerBody::Pipeline(vec![
                    Stage {
                        name: "circle".into(),
                        args: vec![],
                    },
                    Stage {
                        name: "glow".into(),
                        args: vec![],
                    },
                ]),
            }],
            arcs: vec![],
            resonates: vec![crate::ast::ResonateBlock {
                entries: vec![crate::ast::ResonateEntry {
                    source: "bass".into(),
                    target: "core".into(),
                    field: "scale".into(),
                    weight: Expr::Number(0.3),
                }],
            }],
            listen: None,
            voice: None,
            score: None,
            gravity: None,
        };
        let output = generate(&cin).unwrap();
        assert!(output
            .js_modules
            .iter()
            .any(|m| m.contains("GameResonanceNetwork")));
    }

    #[test]
    fn generate_with_arc_produces_js_module() {
        let cin = Cinematic {
            name: "evolving".into(),
            layers: vec![Layer {
                name: "main".into(),
                opts: vec![],
                memory: None,
                cast: None,
                body: LayerBody::Pipeline(vec![
                    Stage {
                        name: "circle".into(),
                        args: vec![],
                    },
                    Stage {
                        name: "glow".into(),
                        args: vec![],
                    },
                ]),
            }],
            arcs: vec![crate::ast::ArcBlock {
                entries: vec![crate::ast::ArcEntry {
                    target: "scale".into(),
                    from: Expr::Number(0.1),
                    to: Expr::Number(1.0),
                    duration: crate::ast::Duration::Seconds(3.0),
                    easing: Some("ease-out".into()),
                }],
            }],
            resonates: vec![],
            listen: None,
            voice: None,
            score: None,
            gravity: None,
        };
        let output = generate(&cin).unwrap();
        assert!(output
            .js_modules
            .iter()
            .any(|m| m.contains("GameArcTimeline")));
        assert!(output.js_modules.iter().any(|m| m.contains("ease_out")));
    }

    #[test]
    fn generate_with_resonate_and_arc_together() {
        let cin = Cinematic {
            name: "living".into(),
            layers: vec![Layer {
                name: "main".into(),
                opts: vec![],
                memory: None,
                cast: None,
                body: LayerBody::Pipeline(vec![
                    Stage {
                        name: "circle".into(),
                        args: vec![],
                    },
                    Stage {
                        name: "glow".into(),
                        args: vec![],
                    },
                ]),
            }],
            arcs: vec![crate::ast::ArcBlock {
                entries: vec![crate::ast::ArcEntry {
                    target: "scale".into(),
                    from: Expr::Number(0.0),
                    to: Expr::Number(1.0),
                    duration: crate::ast::Duration::Seconds(5.0),
                    easing: None,
                }],
            }],
            resonates: vec![crate::ast::ResonateBlock {
                entries: vec![crate::ast::ResonateEntry {
                    source: "scale".into(),
                    target: "core".into(),
                    field: "brightness".into(),
                    weight: Expr::Number(0.5),
                }],
            }],
            listen: None,
            voice: None,
            score: None,
            gravity: None,
        };
        let output = generate(&cin).unwrap();
        let has_resonate = output
            .js_modules
            .iter()
            .any(|m| m.contains("GameResonanceNetwork"));
        let has_arc = output
            .js_modules
            .iter()
            .any(|m| m.contains("GameArcTimeline"));
        assert!(has_resonate, "Should have resonance network");
        assert!(has_arc, "Should have arc timeline");
    }
}
