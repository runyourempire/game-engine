//! TypeScript definition generation for GAME Web Components.
//!
//! Generates `.d.ts` files alongside the compiled `.js` output, providing
//! full type safety for consumers of GAME components.

use crate::codegen::ShaderOutput;

use super::component::{to_kebab, to_pascal};

/// Generate a `.d.ts` TypeScript definition file for a compiled GAME component.
pub fn generate_typescript_defs(shader: &ShaderOutput) -> String {
    let tag = to_kebab(&shader.name);
    let class = to_pascal(&shader.name);
    let interface_name = format!("Game{class}Element");

    let mut s = String::with_capacity(2048);

    // Header
    s.push_str(&format!(
        "/**\n * GAME Component: {tag}\n * Auto-generated TypeScript definitions \u{2014} do not edit.\n */\n\n"
    ));

    // Shared interfaces
    s.push_str("/** Audio data for reactive components. */\n");
    s.push_str("interface GameAudioData {\n");
    s.push_str("  bass: number;\n");
    s.push_str("  mid: number;\n");
    s.push_str("  treble: number;\n");
    s.push_str("  energy: number;\n");
    s.push_str("  beat: number;\n");
    s.push_str("}\n\n");

    s.push_str("/** Audio bridge for subscribable audio sources. */\n");
    s.push_str("interface GameAudioBridge {\n");
    s.push_str("  subscribe(callback: (data: GameAudioData) => void): void;\n");
    s.push_str("}\n\n");

    // Build example attributes string from uniforms
    let example_attrs: String = shader
        .uniforms
        .iter()
        .take(3)
        .map(|u| format!(" {}=\"{}\"", u.name, u.default))
        .collect();

    // Build example property-set lines
    let example_props: String = shader
        .uniforms
        .iter()
        .take(2)
        .map(|u| format!("   * el.{} = {};\n", u.name, u.default))
        .collect();

    // Component interface with JSDoc
    s.push_str(&format!(
        "/**\n * `<game-{tag}>` Web Component\n *\n * A self-contained WebGPU/WebGL2 shader component.\n *\n"
    ));
    if !shader.uniforms.is_empty() {
        s.push_str(" * @example\n * ```html\n");
        s.push_str(&format!(" * <game-{tag}{example_attrs}></game-{tag}>\n"));
        s.push_str(" * ```\n *\n");
        if !example_props.is_empty() {
            s.push_str(" * @example\n * ```typescript\n");
            s.push_str(&format!(
                " * const el = document.querySelector('game-{tag}')!;\n"
            ));
            s.push_str(&example_props);
            s.push_str(" * ```\n");
        }
    }
    s.push_str(" */\n");

    s.push_str(&format!(
        "interface {interface_name} extends HTMLElement {{\n"
    ));

    // Methods
    s.push_str("  /** Set a uniform parameter by name. */\n");
    s.push_str("  setParam(name: string, value: number): void;\n\n");

    s.push_str("  /** Feed audio frequency data for reactive components. */\n");
    s.push_str("  setAudioData(data: GameAudioData): void;\n\n");

    s.push_str("  /** Connect an audio bridge for automatic audio feeding. */\n");
    s.push_str("  setAudioSource(bridge: GameAudioBridge): void;\n\n");

    s.push_str("  /** Capture the current frame as ImageData. */\n");
    s.push_str("  getFrame(): ImageData | null;\n\n");

    s.push_str("  /** Capture the current frame as a data URL (PNG). */\n");
    s.push_str("  getFrameDataURL(type?: string): string | null;\n\n");

    // Uniform properties
    if !shader.uniforms.is_empty() {
        s.push_str("  // Uniform properties\n");
        for u in &shader.uniforms {
            s.push_str(&format!("  /** Default: {} */\n", u.default));
            s.push_str(&format!("  {}: number;\n", u.name));
        }
    }

    // Convenience alias: progress (fill_angle -> progress)
    let has_fill_angle = shader.uniforms.iter().any(|u| u.name == "fill_angle");
    let has_progress = shader.uniforms.iter().any(|u| u.name == "progress");
    if has_fill_angle && !has_progress {
        s.push_str("  /** Convenience alias for fill_angle (0-1 range, mapped to radians). */\n");
        s.push_str("  progress: number;\n");
    }

    // Convenience alias: health (intensity -> health)
    let has_intensity = shader.uniforms.iter().any(|u| u.name == "intensity");
    let has_health = shader.uniforms.iter().any(|u| u.name == "health");
    if has_intensity && !has_health {
        s.push_str("  /** Convenience alias for intensity. */\n");
        s.push_str("  health: number;\n");
    }

    // String properties (from props block)
    if !shader.string_props.is_empty() {
        s.push_str("\n  // String properties\n");
        s.push_str("  /** Set a string property by name. */\n");
        s.push_str("  setStringProp(name: string, value: string): void;\n\n");
        for sp in &shader.string_props {
            s.push_str(&format!(
                "  /** Default: \"{}\" */\n",
                sp.default
            ));
            s.push_str(&format!("  {}: string;\n", sp.name));
        }
    }

    // Event handlers
    if !shader.event_handlers.is_empty() {
        s.push_str("\n  // Custom events\n");
        for (_, emit) in &shader.event_handlers {
            if let Some(name) = emit {
                s.push_str(&format!(
                    "  addEventListener(type: '{}', listener: (ev: CustomEvent) => void, options?: boolean | AddEventListenerOptions): void;\n",
                    name
                ));
            }
        }
    }

    s.push_str("}\n\n");

    // Global augmentation
    s.push_str("declare global {\n");
    s.push_str("  interface HTMLElementTagNameMap {\n");
    s.push_str(&format!("    'game-{tag}': {interface_name};\n"));
    s.push_str("  }\n");
    s.push_str("}\n\n");

    s.push_str("export {};\n");

    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codegen::{ShaderOutput, UniformInfo};

    fn make_shader(name: &str) -> ShaderOutput {
        ShaderOutput {
            name: name.into(),
            wgsl_fragment: String::new(),
            wgsl_vertex: String::new(),
            glsl_fragment: String::new(),
            glsl_vertex: String::new(),
            uniforms: vec![],
            uses_memory: false,
            js_modules: vec![],
            compute_wgsl: None,
            react_wgsl: None,
            swarm_agent_wgsl: None,
            swarm_trail_wgsl: None,
            flow_wgsl: None,
            pass_wgsl: vec![],
            pass_count: 0,
            uses_feedback: false,
            has_coupling_matrix: false,
            string_props: vec![],
            dom_html: None,
            dom_css: None,
            event_handlers: vec![],
            aria_role: None,
            has_arc_enter: false,
            has_arc_exit: false,
            has_arc_hover: false,
        }
    }

    #[test]
    fn dts_has_correct_tag_name() {
        let shader = make_shader("glowing-orb");
        let dts = generate_typescript_defs(&shader);
        assert!(dts.contains("'game-glowing-orb': GameGlowingOrbElement"));
        assert!(dts.contains("interface GameGlowingOrbElement extends HTMLElement"));
    }

    #[test]
    fn dts_includes_shared_interfaces() {
        let shader = make_shader("test");
        let dts = generate_typescript_defs(&shader);
        assert!(dts.contains("interface GameAudioData"));
        assert!(dts.contains("interface GameAudioBridge"));
    }

    #[test]
    fn dts_includes_methods() {
        let shader = make_shader("test");
        let dts = generate_typescript_defs(&shader);
        assert!(dts.contains("setParam(name: string, value: number): void;"));
        assert!(dts.contains("setAudioData(data: GameAudioData): void;"));
        assert!(dts.contains("setAudioSource(bridge: GameAudioBridge): void;"));
        assert!(dts.contains("getFrame(): ImageData | null;"));
        assert!(dts.contains("getFrameDataURL(type?: string): string | null;"));
    }

    #[test]
    fn dts_includes_uniform_properties() {
        let mut shader = make_shader("orb");
        shader.uniforms = vec![
            UniformInfo {
                name: "radius".into(),
                default: 0.2,
            },
            UniformInfo {
                name: "intensity".into(),
                default: 1.5,
            },
        ];
        let dts = generate_typescript_defs(&shader);
        assert!(dts.contains("radius: number;"));
        assert!(dts.contains("intensity: number;"));
        assert!(dts.contains("/** Default: 0.2 */"));
        assert!(dts.contains("/** Default: 1.5 */"));
    }

    #[test]
    fn dts_has_global_augmentation() {
        let shader = make_shader("my-viz");
        let dts = generate_typescript_defs(&shader);
        assert!(dts.contains("declare global {"));
        assert!(dts.contains("interface HTMLElementTagNameMap {"));
        assert!(dts.contains("export {};"));
    }

    #[test]
    fn dts_progress_alias_when_fill_angle_present() {
        let mut shader = make_shader("ring");
        shader.uniforms = vec![UniformInfo {
            name: "fill_angle".into(),
            default: 0.0,
        }];
        let dts = generate_typescript_defs(&shader);
        assert!(
            dts.contains("progress: number;"),
            "should have progress alias"
        );
    }

    #[test]
    fn dts_no_progress_alias_when_progress_uniform_exists() {
        let mut shader = make_shader("countdown");
        shader.uniforms = vec![
            UniformInfo {
                name: "fill_angle".into(),
                default: 0.0,
            },
            UniformInfo {
                name: "progress".into(),
                default: 0.0,
            },
        ];
        let dts = generate_typescript_defs(&shader);
        // Should only have one progress: the uniform, not the alias
        let count = dts.matches("progress: number;").count();
        assert_eq!(count, 1, "expected exactly one progress property");
    }

    #[test]
    fn dts_health_alias_when_intensity_present() {
        let mut shader = make_shader("orb");
        shader.uniforms = vec![UniformInfo {
            name: "intensity".into(),
            default: 1.0,
        }];
        let dts = generate_typescript_defs(&shader);
        assert!(dts.contains("health: number;"), "should have health alias");
    }

    #[test]
    fn dts_no_health_alias_when_health_uniform_exists() {
        let mut shader = make_shader("bar");
        shader.uniforms = vec![
            UniformInfo {
                name: "intensity".into(),
                default: 1.0,
            },
            UniformInfo {
                name: "health".into(),
                default: 1.0,
            },
        ];
        let dts = generate_typescript_defs(&shader);
        // Should only have one health: the uniform, not the alias
        let count = dts.matches("health: number;").count();
        assert_eq!(count, 1, "expected exactly one health property");
    }

    #[test]
    fn dts_no_aliases_without_triggering_uniforms() {
        let mut shader = make_shader("plain");
        shader.uniforms = vec![UniformInfo {
            name: "speed".into(),
            default: 1.0,
        }];
        let dts = generate_typescript_defs(&shader);
        assert!(!dts.contains("progress: number;"));
        assert!(!dts.contains("health: number;"));
    }

    #[test]
    fn dts_header_comment() {
        let shader = make_shader("cool-viz");
        let dts = generate_typescript_defs(&shader);
        assert!(dts.contains("GAME Component: cool-viz"));
        assert!(dts.contains("Auto-generated TypeScript definitions"));
    }
}
