//! Memory (ping-pong framebuffer) codegen for stateful layers.
//!
//! When a layer has `memory: <decay>`, the fragment shader samples the
//! previous frame and mixes it with the current output:
//!   `output = mix(current, prev, decay)`
//!
//! This requires additional bindings (texture + sampler) and runtime
//! support for two render targets swapped each frame.

/// Check if any layer in a cinematic uses memory.
pub fn any_layer_uses_memory(layers: &[crate::ast::Layer]) -> bool {
    layers.iter().any(|l| l.memory.is_some())
}

/// Emit WGSL bind group declarations for memory texture (Group 1).
pub fn emit_wgsl_memory_bindings(s: &mut String) {
    s.push_str("@group(1) @binding(0) var prev_frame: texture_2d<f32>;\n");
    s.push_str("@group(1) @binding(1) var prev_sampler: sampler;\n\n");
}

/// Emit WGSL code to sample previous frame and mix with current color.
/// Called at the end of a layer that has `memory: <decay>`.
pub fn emit_wgsl_memory_mix(s: &mut String, decay: f64, indent: &str) {
    s.push_str(&format!(
        "{indent}let prev_color = textureSample(prev_frame, prev_sampler, input.uv);\n"
    ));
    s.push_str(&format!(
        "{indent}color_result = mix(color_result, prev_color, {decay:.6});\n"
    ));
}

/// Emit GLSL uniform declarations for memory texture.
pub fn emit_glsl_memory_bindings(s: &mut String) {
    s.push_str("uniform sampler2D u_prev_frame;\n\n");
}

/// Emit GLSL code to sample previous frame and mix with current color.
pub fn emit_glsl_memory_mix(s: &mut String, decay: f64, indent: &str) {
    s.push_str(&format!(
        "{indent}vec4 prev_color = texture(u_prev_frame, v_uv);\n"
    ));
    s.push_str(&format!(
        "{indent}color_result = mix(color_result, prev_color, {decay:.6});\n"
    ));
}

/// Generate the JS runtime code for WebGPU ping-pong texture management.
pub fn webgpu_memory_runtime() -> &'static str {
    r#"  _initMemory() {
    const w = this.canvas.width || 1;
    const h = this.canvas.height || 1;
    const desc = {
      size: { width: w, height: h },
      format: 'rgba8unorm',
      usage: GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.COPY_SRC | GPUTextureUsage.COPY_DST
    };
    this._memTex = [this.device.createTexture(desc), this.device.createTexture(desc)];
    this._memIdx = 0;
    this._memSampler = this.device.createSampler({ magFilter: 'linear', minFilter: 'linear' });
    this._memBindGroupLayout = this.device.createBindGroupLayout({
      entries: [
        { binding: 0, visibility: GPUShaderStage.FRAGMENT, texture: { sampleType: 'float' } },
        { binding: 1, visibility: GPUShaderStage.FRAGMENT, sampler: { type: 'filtering' } }
      ]
    });
    this._updateMemBindGroup();
  }

  _updateMemBindGroup() {
    const readTex = this._memTex[this._memIdx];
    this._memBindGroup = this.device.createBindGroup({
      layout: this._memBindGroupLayout,
      entries: [
        { binding: 0, resource: readTex.createView() },
        { binding: 1, resource: this._memSampler }
      ]
    });
  }

  _swapMemory(encoder) {
    const writeTex = this._memTex[1 - this._memIdx];
    // Copy current frame to write texture for next frame's read
    encoder.copyTextureToTexture(
      { texture: this.ctx.getCurrentTexture() },
      { texture: writeTex },
      { width: this.canvas.width, height: this.canvas.height }
    );
    this._memIdx = 1 - this._memIdx;
    this._updateMemBindGroup();
  }

  _resizeMemory() {
    if (this._memTex) {
      this._memTex[0].destroy();
      this._memTex[1].destroy();
      this._initMemory();
    }
  }"#
}

/// Generate the JS runtime code for WebGL2 ping-pong FBO management.
pub fn webgl2_memory_runtime() -> &'static str {
    r#"  _initMemoryGL() {
    const gl = this.gl;
    const w = this.canvas.width || 1;
    const h = this.canvas.height || 1;
    this._memFbo = [gl.createFramebuffer(), gl.createFramebuffer()];
    this._memTex = [gl.createTexture(), gl.createTexture()];
    for (let i = 0; i < 2; i++) {
      gl.bindTexture(gl.TEXTURE_2D, this._memTex[i]);
      gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, w, h, 0, gl.RGBA, gl.UNSIGNED_BYTE, null);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
      gl.bindFramebuffer(gl.FRAMEBUFFER, this._memFbo[i]);
      gl.framebufferTexture2D(gl.FRAMEBUFFER, gl.COLOR_ATTACHMENT0, gl.TEXTURE_2D, this._memTex[i], 0);
    }
    gl.bindFramebuffer(gl.FRAMEBUFFER, null);
    gl.bindTexture(gl.TEXTURE_2D, null);
    this._memIdx = 0;
    this._memLoc = gl.getUniformLocation(this.program, 'u_prev_frame');
  }

  _swapMemoryGL() {
    const gl = this.gl;
    // Bind read texture to unit 1 for next frame
    gl.activeTexture(gl.TEXTURE1);
    gl.bindTexture(gl.TEXTURE_2D, this._memTex[this._memIdx]);
    gl.uniform1i(this._memLoc, 1);
    // Render to write FBO, then copy to screen
    this._memIdx = 1 - this._memIdx;
  }

  _resizeMemoryGL() {
    if (this._memTex) {
      const gl = this.gl;
      const w = this.canvas.width || 1;
      const h = this.canvas.height || 1;
      for (let i = 0; i < 2; i++) {
        gl.bindTexture(gl.TEXTURE_2D, this._memTex[i]);
        gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, w, h, 0, gl.RGBA, gl.UNSIGNED_BYTE, null);
      }
      gl.bindTexture(gl.TEXTURE_2D, null);
    }
  }"#
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::*;

    #[test]
    fn detects_memory_layers() {
        let layers = vec![
            Layer {
                name: "a".into(),
                opts: vec![],
                memory: None,
                cast: None,
                body: LayerBody::Pipeline(vec![]),
            },
            Layer {
                name: "b".into(),
                opts: vec![],
                memory: Some(0.95),
                cast: None,
                body: LayerBody::Pipeline(vec![]),
            },
        ];
        assert!(any_layer_uses_memory(&layers));
    }

    #[test]
    fn no_memory_when_absent() {
        let layers = vec![Layer {
            name: "a".into(),
            opts: vec![],
            memory: None,
            cast: None,
            body: LayerBody::Pipeline(vec![]),
        }];
        assert!(!any_layer_uses_memory(&layers));
    }

    #[test]
    fn wgsl_memory_mix_emits_correct_code() {
        let mut s = String::new();
        emit_wgsl_memory_mix(&mut s, 0.97, "    ");
        assert!(s.contains("textureSample(prev_frame, prev_sampler, input.uv)"));
        assert!(s.contains("mix(color_result, prev_color, 0.970000)"));
    }

    #[test]
    fn glsl_memory_mix_emits_correct_code() {
        let mut s = String::new();
        emit_glsl_memory_mix(&mut s, 0.95, "    ");
        assert!(s.contains("texture(u_prev_frame, v_uv)"));
        assert!(s.contains("mix(color_result, prev_color, 0.950000)"));
    }

    #[test]
    fn wgsl_bindings_has_group_1() {
        let mut s = String::new();
        emit_wgsl_memory_bindings(&mut s);
        assert!(s.contains("@group(1) @binding(0)"));
        assert!(s.contains("@group(1) @binding(1)"));
    }
}
