//! 3D SDF ray marching codegen.
//!
//! When `scene3d` is present, generates a sphere-tracing fragment shader
//! with Phong lighting instead of the standard 2D UV projection.

use crate::ast::*;
use crate::codegen::UniformInfo;

/// Generate a 3D ray-marched fragment shader from the cinematic's layers.
///
/// Uses sphere-tracing (ray marching) with SDF scene evaluation,
/// normal estimation via central differences, and Phong lighting.
pub fn generate_fragment_3d(cinematic: &Cinematic, uniforms: &[UniformInfo]) -> String {
    let scene3d = cinematic.scene3d.as_ref().expect("scene3d required for 3D mode");
    let fov = scene3d.fov;
    let distance = scene3d.distance;

    let mut s = String::with_capacity(4096);

    // Uniform struct (same as 2D)
    s.push_str("struct Uniforms {\n");
    s.push_str("    time: f32,\n");
    s.push_str("    audio_bass: f32,\n    audio_mid: f32,\n    audio_treble: f32,\n");
    s.push_str("    audio_energy: f32,\n    audio_beat: f32,\n");
    s.push_str("    resolution: vec2<f32>,\n");
    s.push_str("    mouse: vec2<f32>,\n");
    s.push_str("    mouse_down: f32,\n");
    s.push_str("    aspect_ratio: f32,\n");
    for u in uniforms {
        s.push_str(&format!("    {}: f32,\n", u.name));
    }
    s.push_str("};\n\n");

    s.push_str("struct VertexOutput {\n");
    s.push_str("    @builtin(position) pos: vec4<f32>,\n");
    s.push_str("    @location(0) uv: vec2<f32>,\n");
    s.push_str("};\n\n");

    s.push_str("@group(0) @binding(0) var<uniform> u: Uniforms;\n\n");

    // 3D SDF primitives
    s.push_str("fn sdf_sphere_3d(p: vec3<f32>, radius: f32) -> f32 {\n");
    s.push_str("    return length(p) - radius;\n");
    s.push_str("}\n\n");

    s.push_str("fn sdf_box_3d(p: vec3<f32>, b: vec3<f32>) -> f32 {\n");
    s.push_str("    let q = abs(p) - b;\n");
    s.push_str("    return length(max(q, vec3<f32>(0.0))) + min(max(q.x, max(q.y, q.z)), 0.0);\n");
    s.push_str("}\n\n");

    s.push_str("fn sdf_torus_3d(p: vec3<f32>, major: f32, minor: f32) -> f32 {\n");
    s.push_str("    let q = vec2<f32>(length(p.xz) - major, p.y);\n");
    s.push_str("    return length(q) - minor;\n");
    s.push_str("}\n\n");

    s.push_str("fn sdf_cylinder_3d(p: vec3<f32>, radius: f32, height: f32) -> f32 {\n");
    s.push_str("    let d = abs(vec2<f32>(length(p.xz), p.y)) - vec2<f32>(radius, height);\n");
    s.push_str("    return min(max(d.x, d.y), 0.0) + length(max(d, vec2<f32>(0.0)));\n");
    s.push_str("}\n\n");

    // Scene SDF — combine all layers
    // For now, use a default sphere if layers don't have 3D-specific stages
    s.push_str("fn scene_sdf(p: vec3<f32>) -> f32 {\n");
    if cinematic.layers.is_empty() {
        s.push_str("    return sdf_sphere_3d(p, 0.5);\n");
    } else {
        // Use the first layer's first stage to determine the shape
        let first_layer = &cinematic.layers[0];
        let shape = match &first_layer.body {
            LayerBody::Pipeline(stages) if !stages.is_empty() => {
                match stages[0].name.as_str() {
                    "box" => {
                        let w = stages[0].args.get(0).map_or(0.3, |a| match &a.value { Expr::Number(v) => *v, _ => 0.3 });
                        let h = stages[0].args.get(1).map_or(0.2, |a| match &a.value { Expr::Number(v) => *v, _ => 0.2 });
                        format!("sdf_box_3d(p, vec3<f32>({w}, {h}, {w}))")
                    }
                    _ => {
                        let r = stages[0].args.get(0).map_or(0.5, |a| match &a.value { Expr::Number(v) => *v, _ => 0.5 });
                        format!("sdf_sphere_3d(p, {r})")
                    }
                }
            }
            _ => "sdf_sphere_3d(p, 0.5)".to_string(),
        };
        s.push_str(&format!("    return {shape};\n"));
    }
    s.push_str("}\n\n");

    // Normal estimation
    s.push_str("fn estimate_normal(p: vec3<f32>) -> vec3<f32> {\n");
    s.push_str("    let e = 0.001;\n");
    s.push_str("    return normalize(vec3<f32>(\n");
    s.push_str("        scene_sdf(p + vec3<f32>(e, 0.0, 0.0)) - scene_sdf(p - vec3<f32>(e, 0.0, 0.0)),\n");
    s.push_str("        scene_sdf(p + vec3<f32>(0.0, e, 0.0)) - scene_sdf(p - vec3<f32>(0.0, e, 0.0)),\n");
    s.push_str("        scene_sdf(p + vec3<f32>(0.0, 0.0, e)) - scene_sdf(p - vec3<f32>(0.0, 0.0, e)),\n");
    s.push_str("    ));\n");
    s.push_str("}\n\n");

    // Camera
    let orbit = matches!(scene3d.camera, CameraMode::Orbit);
    s.push_str("@fragment\n");
    s.push_str("fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {\n");
    s.push_str("    let uv = input.uv * 2.0 - 1.0;\n");
    s.push_str("    let aspect = u.aspect_ratio;\n\n");

    // Ray origin & direction
    s.push_str(&format!("    let fov_rad = {} * 0.01745329;\n", fov));
    s.push_str("    let focal = 1.0 / tan(fov_rad * 0.5);\n");
    s.push_str("    let rd_cam = normalize(vec3<f32>(uv.x * aspect, uv.y, -focal));\n\n");

    if orbit {
        s.push_str("    // Orbit camera — rotate around Y axis with time + mouse\n");
        s.push_str("    let angle_y = u.time * 0.3 + u.mouse.x * 3.14159;\n");
        s.push_str("    let angle_x = u.mouse.y * 1.5 - 0.3;\n");
        s.push_str("    let cy = cos(angle_y); let sy = sin(angle_y);\n");
        s.push_str("    let cx = cos(angle_x); let sx = sin(angle_x);\n");
        s.push_str("    let rd = vec3<f32>(\n");
        s.push_str("        cy * rd_cam.x + sy * rd_cam.z,\n");
        s.push_str("        cx * rd_cam.y - sx * (cy * rd_cam.z - sy * rd_cam.x),\n");
        s.push_str("        sx * rd_cam.y + cx * (cy * rd_cam.z - sy * rd_cam.x),\n");
        s.push_str("    );\n");
        s.push_str(&format!("    let ro = vec3<f32>(sy * {distance}, sx * {distance} * 0.5, cy * {distance});\n\n"));
    } else {
        s.push_str(&format!("    let ro = vec3<f32>(0.0, 0.0, {distance});\n"));
        s.push_str("    let rd = rd_cam;\n\n");
    }

    // Ray march loop
    s.push_str("    var t: f32 = 0.0;\n");
    s.push_str("    var hit = false;\n");
    s.push_str("    for (var i: u32 = 0u; i < 100u; i = i + 1u) {\n");
    s.push_str("        let p = ro + rd * t;\n");
    s.push_str("        let d = scene_sdf(p);\n");
    s.push_str("        if (d < 0.001) { hit = true; break; }\n");
    s.push_str("        if (t > 20.0) { break; }\n");
    s.push_str("        t += d;\n");
    s.push_str("    }\n\n");

    // Lighting
    s.push_str("    if (!hit) { return vec4<f32>(0.0, 0.0, 0.0, 1.0); }\n\n");
    s.push_str("    let pos = ro + rd * t;\n");
    s.push_str("    let normal = estimate_normal(pos);\n");
    s.push_str("    let light_dir = normalize(vec3<f32>(0.5, 0.8, 0.6));\n\n");

    // Extract color from layers (first layer's shade/tint)
    let (cr, cg, cb) = extract_color_from_layers(cinematic);

    s.push_str("    // Phong lighting\n");
    s.push_str("    let ambient = 0.15;\n");
    s.push_str("    let diffuse = max(dot(normal, light_dir), 0.0);\n");
    s.push_str("    let view_dir = normalize(-rd);\n");
    s.push_str("    let half_dir = normalize(light_dir + view_dir);\n");
    s.push_str("    let specular = pow(max(dot(normal, half_dir), 0.0), 32.0);\n\n");
    s.push_str(&format!(
        "    let base_color = vec3<f32>({cr}, {cg}, {cb});\n"
    ));
    s.push_str("    let lit = base_color * (ambient + diffuse * 0.7) + vec3<f32>(1.0) * specular * 0.3;\n");
    s.push_str("    return vec4<f32>(lit, 1.0);\n");
    s.push_str("}\n");

    s
}

/// Extract base color from the cinematic's layers (tint or shade args).
fn extract_color_from_layers(cinematic: &Cinematic) -> (f64, f64, f64) {
    for layer in &cinematic.layers {
        if let LayerBody::Pipeline(stages) = &layer.body {
            for stage in stages {
                if stage.name == "shade" || stage.name == "tint" {
                    let r = stage.args.get(0).map_or(1.0, |a| match &a.value { Expr::Number(v) => *v, _ => 1.0 });
                    let g = stage.args.get(1).map_or(1.0, |a| match &a.value { Expr::Number(v) => *v, _ => 1.0 });
                    let b = stage.args.get(2).map_or(1.0, |a| match &a.value { Expr::Number(v) => *v, _ => 1.0 });
                    return (r, g, b);
                }
            }
        }
    }
    (1.0, 1.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_3d_cinematic() -> (Cinematic, Vec<UniformInfo>) {
        let cin = Cinematic {
            name: "3d-test".into(),
            layers: vec![Layer {
                name: "main".into(),
                opts: vec![],
                memory: None,
                opacity: None,
                cast: None,
                blend: BlendMode::Add,
                feedback: false,
                body: LayerBody::Pipeline(vec![
                    Stage { name: "sphere".into(), args: vec![Arg { name: None, value: Expr::Number(0.5) }] },
                    Stage { name: "shade".into(), args: vec![
                        Arg { name: None, value: Expr::Number(0.83) },
                        Arg { name: None, value: Expr::Number(0.69) },
                        Arg { name: None, value: Expr::Number(0.22) },
                    ]},
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
            particles: None,
            passes: vec![],
            cinematic_uses: vec![],
            matrix_coupling: None,
            matrix_color: None,
            props: None,
            dom: None,
            events: vec![],
            role: None,
            scene3d: Some(Scene3dBlock {
                camera: CameraMode::Orbit,
                fov: 45.0,
                distance: 3.0,
            }),
            textures: vec![],
        };
        (cin, vec![])
    }

    #[test]
    fn generates_ray_march_loop() {
        let (cin, uniforms) = make_3d_cinematic();
        let wgsl = generate_fragment_3d(&cin, &uniforms);
        assert!(wgsl.contains("fn fs_main"));
        assert!(wgsl.contains("scene_sdf"));
        assert!(wgsl.contains("estimate_normal"));
        assert!(wgsl.contains("for (var i: u32 = 0u; i < 100u"));
    }

    #[test]
    fn has_3d_sdf_primitives() {
        let (cin, uniforms) = make_3d_cinematic();
        let wgsl = generate_fragment_3d(&cin, &uniforms);
        assert!(wgsl.contains("fn sdf_sphere_3d"));
        assert!(wgsl.contains("fn sdf_box_3d"));
        assert!(wgsl.contains("fn sdf_torus_3d"));
    }

    #[test]
    fn has_phong_lighting() {
        let (cin, uniforms) = make_3d_cinematic();
        let wgsl = generate_fragment_3d(&cin, &uniforms);
        assert!(wgsl.contains("ambient"));
        assert!(wgsl.contains("diffuse"));
        assert!(wgsl.contains("specular"));
    }

    #[test]
    fn extracts_color_from_shade() {
        let (cin, uniforms) = make_3d_cinematic();
        let wgsl = generate_fragment_3d(&cin, &uniforms);
        assert!(wgsl.contains("0.83"));
        assert!(wgsl.contains("0.69"));
        assert!(wgsl.contains("0.22"));
    }

    #[test]
    fn orbit_camera_uses_mouse() {
        let (cin, uniforms) = make_3d_cinematic();
        let wgsl = generate_fragment_3d(&cin, &uniforms);
        assert!(wgsl.contains("u.mouse.x"));
        assert!(wgsl.contains("angle_y"));
    }
}
