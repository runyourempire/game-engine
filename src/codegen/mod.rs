//! Shader codegen orchestration.
//!
//! Generates WGSL and/or GLSL shaders from GAME AST, then hands off to
//! the runtime module to wrap them in Web Components or standalone HTML.

pub mod arc;
pub mod automaton;
pub mod breed;
pub mod cast;
pub mod flow;
pub mod glsl;
pub mod gravity;
pub mod ifs;
pub mod listen;
pub mod lsystem;
pub mod matrix;
pub mod memory;
pub mod project;
pub mod react;
pub mod resonate;
pub mod scene;
pub mod score;
pub mod stages;
pub mod swarm;
pub mod temporal;
pub mod voice;
pub mod wgsl;

use crate::ast::{Cinematic, Expr, FnDef, LayerBody, Param};
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
    /// Collected JS classes (listen, voice, score, breed, temporal, gravity, react, swarm, flow).
    pub js_modules: Vec<String>,
    /// Gravity compute shader (separate pipeline).
    pub compute_wgsl: Option<String>,
    /// Reaction-diffusion compute shader.
    pub react_wgsl: Option<String>,
    /// Swarm agent compute shader.
    pub swarm_agent_wgsl: Option<String>,
    /// Swarm trail diffuse/decay compute shader.
    pub swarm_trail_wgsl: Option<String>,
    /// Flow field compute shader.
    pub flow_wgsl: Option<String>,
    /// Post-processing pass fragment shaders (ordered).
    pub pass_wgsl: Vec<String>,
    /// Number of post-processing passes.
    pub pass_count: usize,
    /// Whether this cinematic uses feedback (previous frame as input).
    pub uses_feedback: bool,
    /// Whether this cinematic has a coupling matrix (CPU-side parameter propagation).
    pub has_coupling_matrix: bool,
}

/// Returns true if the name is a built-in shader variable (time, mouse, audio).
fn is_builtin_variable(name: &str) -> bool {
    matches!(
        name,
        "time"
            | "mouse_x"
            | "mouse_y"
            | "mouse_down"
            | "bass"
            | "mid"
            | "treble"
            | "energy"
            | "beat"
    )
}

/// Returns true if the name is a named palette (fire, ocean, aurora, etc.).
fn is_palette_name(name: &str) -> bool {
    matches!(
        name,
        "fire"
            | "ocean"
            | "neon"
            | "aurora"
            | "sunset"
            | "ice"
            | "ember"
            | "lava"
            | "magma"
            | "inferno"
            | "plasma"
            | "electric"
            | "cyber"
            | "matrix"
            | "forest"
            | "moss"
            | "earth"
            | "desert"
            | "blood"
            | "rose"
            | "candy"
            | "royal"
            | "deep_sea"
            | "coral"
            | "arctic"
            | "twilight"
            | "vapor"
            | "gold"
            | "silver"
            | "monochrome"
    )
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

        // Pipeline stages: ident args that aren't builtins/palettes/built-in vars are user uniforms
        let stages_to_scan: Vec<&crate::ast::Stage> = match &layer.body {
            LayerBody::Pipeline(stages) => stages.iter().collect(),
            LayerBody::Conditional {
                then_branch,
                else_branch,
                ..
            } => then_branch.iter().chain(else_branch.iter()).collect(),
            _ => vec![],
        };
        for stage in stages_to_scan {
            for arg in &stage.args {
                if let Expr::Ident(name) = &arg.value {
                    if builtins::lookup(name).is_none()
                        && !is_builtin_variable(name)
                        && !is_palette_name(name)
                        && seen.insert(name.clone())
                    {
                        uniforms.push(UniformInfo {
                            name: name.clone(),
                            default: 0.0,
                        });
                    }
                }
            }
        }
    }

    uniforms
}

/// Validate all pipeline layers in a cinematic.
pub fn validate(cinematic: &Cinematic, fns: &[FnDef]) -> Result<(), CompileError> {
    for layer in &cinematic.layers {
        match &layer.body {
            LayerBody::Pipeline(pipeline) => {
                stages::validate_pipeline_with_fns(pipeline, fns)?;
            }
            LayerBody::Conditional {
                then_branch,
                else_branch,
                ..
            } => {
                let then_state = stages::validate_pipeline_with_fns(then_branch, fns)?;
                let else_state = stages::validate_pipeline_with_fns(else_branch, fns)?;
                if then_state != else_state {
                    return Err(CompileError::validation(format!(
                        "if/else branches produce different states: {:?} vs {:?}",
                        then_state, else_state
                    )));
                }
            }
            LayerBody::Params(_) => {}
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

/// Public accessor for extracting uniform info from a cinematic (used by dev server).
pub fn extract_uniforms_public(cinematic: &Cinematic) -> Vec<UniformInfo> {
    extract_uniforms(cinematic)
}

/// Generate shaders for a single cinematic.
pub fn generate(cinematic: &Cinematic) -> Result<ShaderOutput, CompileError> {
    generate_with_fns(cinematic, &[])
}

/// Generate shaders for a cinematic with user-defined function context.
pub fn generate_with_fns(
    cinematic: &Cinematic,
    fns: &[FnDef],
) -> Result<ShaderOutput, CompileError> {
    validate(cinematic, fns)?;

    let uniforms = extract_uniforms(cinematic);

    let wgsl_fragment = wgsl::generate_fragment_with_fns(cinematic, &uniforms, fns);
    let glsl_fragment = glsl::generate_fragment_with_fns(cinematic, &uniforms, fns);

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

    // Matrix coupling → GameCouplingMatrix JS class
    if let Some(ref mc) = cinematic.matrix_coupling {
        js_modules.push(matrix::generate_coupling_js(mc));
    }

    // Note: transition matrices are top-level (Program.matrix_blocks),
    // not per-cinematic. They are handled separately during scene codegen.
    // Color matrices are injected directly into fragment shaders by wgsl.rs/glsl.rs.

    let has_coupling_matrix = cinematic.matrix_coupling.is_some();

    // React → compute WGSL + GameReactionField JS class
    let react_wgsl = if let Some(ref rb) = cinematic.react {
        let (w, h) = (256u32, 256u32);
        js_modules.push(react::generate_compute_runtime_js(rb, w, h));
        Some(react::generate_compute_wgsl(rb))
    } else {
        None
    };

    // Swarm → dual compute WGSL + GameSwarmSim JS class
    let (swarm_agent_wgsl, swarm_trail_wgsl) = if let Some(ref sb) = cinematic.swarm {
        let (w, h) = (512u32, 512u32);
        js_modules.push(swarm::generate_swarm_runtime_js(sb, w, h));
        (
            Some(swarm::generate_agent_wgsl(sb)),
            Some(swarm::generate_trail_wgsl(sb)),
        )
    } else {
        (None, None)
    };

    // Flow → compute WGSL + GameFlowField JS class
    let flow_wgsl = if let Some(ref fb) = cinematic.flow {
        let (w, h) = (256u32, 256u32);
        js_modules.push(flow::generate_flow_runtime_js(fb, w, h));
        Some(flow::generate_compute_wgsl(fb))
    } else {
        None
    };

    // Pass blocks → additional fragment shaders for post-processing
    let pass_wgsl: Vec<String> = cinematic
        .passes
        .iter()
        .map(|pb| wgsl::generate_pass_fragment(pb))
        .collect();
    let pass_count = pass_wgsl.len();

    // Feedback detection
    let uses_feedback = cinematic.layers.iter().any(|l| l.feedback);

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
        react_wgsl,
        swarm_agent_wgsl,
        swarm_trail_wgsl,
        flow_wgsl,
        pass_wgsl,
        pass_count,
        uses_feedback,
        has_coupling_matrix,
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
                opacity: None,
                cast: None,
                blend: BlendMode::Add,
                feedback: false,
                body: LayerBody::Pipeline(stages),
            }],
            arcs: vec![],
            resonates: vec![],
            listen: None,
            voice: None,
            score: None,
            gravity: None,
            react: None,
            swarm: None,
            flow: None,
            passes: vec![],
            cinematic_uses: vec![],
            matrix_coupling: None,
            matrix_color: None,
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
                opacity: None,
                cast: None,
                blend: BlendMode::Add,
                feedback: false,
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
            react: None,
            swarm: None,
            flow: None,
            passes: vec![],
            cinematic_uses: vec![],
            matrix_coupling: None,
            matrix_color: None,
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
                opacity: None,
                cast: Some("sdf".into()),
                blend: BlendMode::Add,
                feedback: false,
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
            react: None,
            swarm: None,
            flow: None,
            passes: vec![],
            cinematic_uses: vec![],
            matrix_coupling: None,
            matrix_color: None,
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
                opacity: None,
                cast: Some("sdf".into()),
                blend: BlendMode::Add,
                feedback: false,
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
            react: None,
            swarm: None,
            flow: None,
            passes: vec![],
            cinematic_uses: vec![],
            matrix_coupling: None,
            matrix_color: None,
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
                opacity: None,
                cast: None,
                blend: BlendMode::Add,
                feedback: false,
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
            react: None,
            swarm: None,
            flow: None,
            passes: vec![],
            cinematic_uses: vec![],
            matrix_coupling: None,
            matrix_color: None,
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
                opacity: None,
                cast: None,
                blend: BlendMode::Add,
                feedback: false,
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
            react: None,
            swarm: None,
            flow: None,
            passes: vec![],
            cinematic_uses: vec![],
            matrix_coupling: None,
            matrix_color: None,
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
                opacity: None,
                cast: None,
                blend: BlendMode::Add,
                feedback: false,
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
            react: None,
            swarm: None,
            flow: None,
            passes: vec![],
            cinematic_uses: vec![],
            matrix_coupling: None,
            matrix_color: None,
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
                opacity: None,
                cast: None,
                blend: BlendMode::Add,
                feedback: false,
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
            react: None,
            swarm: None,
            flow: None,
            passes: vec![],
            cinematic_uses: vec![],
            matrix_coupling: None,
            matrix_color: None,
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
                opacity: None,
                cast: None,
                blend: BlendMode::Add,
                feedback: false,
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
            react: None,
            swarm: None,
            flow: None,
            passes: vec![],
            cinematic_uses: vec![],
            matrix_coupling: None,
            matrix_color: None,
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

    #[test]
    fn generate_with_react_produces_compute() {
        let cin = Cinematic {
            name: "turing".into(),
            layers: vec![Layer {
                name: "bg".into(),
                opts: vec![],
                memory: None,
                opacity: None,
                cast: None,
                blend: BlendMode::Add,
                feedback: false,
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
            react: Some(crate::ast::ReactBlock {
                feed: 0.055,
                kill: 0.062,
                diffuse_a: 1.0,
                diffuse_b: 0.5,
                seed: crate::ast::SeedMode::Center(0.1),
            }),
            swarm: None,
            flow: None,
            passes: vec![],
            cinematic_uses: vec![],
            matrix_coupling: None,
            matrix_color: None,
        };
        let output = generate(&cin).unwrap();
        assert!(output.react_wgsl.is_some());
        let has_runtime = output
            .js_modules
            .iter()
            .any(|m| m.contains("GameReactionField"));
        assert!(has_runtime, "Should have reaction field runtime");
    }

    #[test]
    fn generate_with_swarm_produces_dual_compute() {
        let cin = Cinematic {
            name: "physarum".into(),
            layers: vec![Layer {
                name: "bg".into(),
                opts: vec![],
                memory: None,
                opacity: None,
                cast: None,
                blend: BlendMode::Add,
                feedback: false,
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
            react: None,
            swarm: Some(crate::ast::SwarmBlock {
                agents: 50000,
                sensor_angle: 45.0,
                sensor_dist: 9.0,
                turn_angle: 45.0,
                step_size: 1.0,
                deposit: 5.0,
                decay: 0.95,
                diffuse: 1,
                bounds: crate::ast::BoundsMode::Wrap,
            }),
            flow: None,
            passes: vec![],
            cinematic_uses: vec![],
            matrix_coupling: None,
            matrix_color: None,
        };
        let output = generate(&cin).unwrap();
        assert!(output.swarm_agent_wgsl.is_some());
        assert!(output.swarm_trail_wgsl.is_some());
        let has_runtime = output.js_modules.iter().any(|m| m.contains("GameSwarmSim"));
        assert!(has_runtime, "Should have swarm sim runtime");
    }

    #[test]
    fn generate_with_flow_produces_compute() {
        let cin = Cinematic {
            name: "smoke".into(),
            layers: vec![Layer {
                name: "bg".into(),
                opts: vec![],
                memory: None,
                opacity: None,
                cast: None,
                blend: BlendMode::Add,
                feedback: false,
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
            react: None,
            swarm: None,
            flow: Some(crate::ast::FlowBlock {
                flow_type: crate::ast::FlowType::Curl,
                scale: 3.0,
                speed: 0.5,
                octaves: 4,
                strength: 1.0,
                bounds: crate::ast::BoundsMode::Wrap,
            }),
            passes: vec![],
            cinematic_uses: vec![],
            matrix_coupling: None,
            matrix_color: None,
        };
        let output = generate(&cin).unwrap();
        assert!(output.flow_wgsl.is_some());
        let has_runtime = output
            .js_modules
            .iter()
            .any(|m| m.contains("GameFlowField"));
        assert!(has_runtime, "Should have flow field runtime");
    }
}
