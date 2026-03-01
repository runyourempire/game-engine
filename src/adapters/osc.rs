//! OSC adapter — generates runtime JS for WebSocket bridge to OSC messages.

/// Generate JavaScript for OSC over WebSocket.
pub fn generate_osc_adapter(host: &str, port: u16, path: &str) -> String {
    let mut s = String::with_capacity(1024);

    s.push_str("class GameOscAdapter {\n");
    s.push_str(&format!(
        "  constructor() {{ this._url = 'ws://{}:{}'; this._path = '/{}'; this._values = {{}}; }}\n",
        host, port, path
    ));

    s.push_str("\n  init() {\n");
    s.push_str("    try {\n");
    s.push_str("      this._ws = new WebSocket(this._url);\n");
    s.push_str("      this._ws.onmessage = (e) => {\n");
    s.push_str("        try {\n");
    s.push_str("          const msg = JSON.parse(e.data);\n");
    s.push_str("          if (msg.address && msg.address.startsWith(this._path)) {\n");
    s.push_str("            const key = msg.address.slice(this._path.length + 1) || 'value';\n");
    s.push_str("            this._values[key] = msg.args?.[0] ?? 0;\n");
    s.push_str("          }\n");
    s.push_str("        } catch(_) {}\n");
    s.push_str("      };\n");
    s.push_str("      return true;\n");
    s.push_str("    } catch(e) { return false; }\n");
    s.push_str("  }\n\n");

    s.push_str("  get(name) { return this._values[name] || 0; }\n");
    s.push_str("  getAll() { return {...this._values}; }\n\n");

    s.push_str("  destroy() {\n");
    s.push_str("    if (this._ws) { this._ws.close(); this._ws = null; }\n");
    s.push_str("  }\n");
    s.push_str("}\n");

    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_osc_adapter() {
        let js = generate_osc_adapter("localhost", 9000, "params");
        assert!(js.contains("class GameOscAdapter"));
        assert!(js.contains("ws://localhost:9000"));
        assert!(js.contains("/params"));
        assert!(js.contains("WebSocket"));
    }
}
