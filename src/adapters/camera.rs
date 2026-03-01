//! Camera adapter — generates runtime JS for getUserMedia texture source.

/// Generate JavaScript for webcam video texture injection.
pub fn generate_camera_adapter(device_index: u32) -> String {
    let mut s = String::with_capacity(1024);

    s.push_str("class GameCameraAdapter {\n");
    s.push_str(&format!(
        "  constructor() {{ this._deviceIdx = {}; this._video = null; this._stream = null; }}\n",
        device_index
    ));

    s.push_str("\n  async init() {\n");
    s.push_str("    try {\n");
    s.push_str("      const devices = await navigator.mediaDevices.enumerateDevices();\n");
    s.push_str("      const cameras = devices.filter(d => d.kind === 'videoinput');\n");
    s.push_str(&format!(
        "      const cam = cameras[{}] || cameras[0];\n", device_index
    ));
    s.push_str("      if (!cam) return false;\n");
    s.push_str("      this._stream = await navigator.mediaDevices.getUserMedia({\n");
    s.push_str("        video: { deviceId: cam.deviceId, width: 512, height: 512 }\n");
    s.push_str("      });\n");
    s.push_str("      this._video = document.createElement('video');\n");
    s.push_str("      this._video.srcObject = this._stream;\n");
    s.push_str("      this._video.play();\n");
    s.push_str("      return true;\n");
    s.push_str("    } catch(e) { return false; }\n");
    s.push_str("  }\n\n");

    s.push_str("  getVideoElement() { return this._video; }\n\n");

    s.push_str("  destroy() {\n");
    s.push_str("    if (this._stream) {\n");
    s.push_str("      this._stream.getTracks().forEach(t => t.stop());\n");
    s.push_str("      this._stream = null;\n");
    s.push_str("    }\n");
    s.push_str("    this._video = null;\n");
    s.push_str("  }\n");
    s.push_str("}\n");

    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_camera_adapter() {
        let js = generate_camera_adapter(0);
        assert!(js.contains("class GameCameraAdapter"));
        assert!(js.contains("getUserMedia"));
        assert!(js.contains("videoinput"));
    }
}
