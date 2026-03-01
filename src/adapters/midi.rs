//! MIDI adapter — generates runtime JS for Web MIDI API signal source.

/// Generate JavaScript for Web MIDI input on a specific channel.
pub fn generate_midi_adapter(channel: u8) -> String {
    let mut s = String::with_capacity(1024);

    s.push_str("class GameMidiAdapter {\n");
    s.push_str(&format!(
        "  constructor() {{ this._channel = {}; this._values = {{}}; }}\n",
        channel
    ));

    s.push_str("\n  async init() {\n");
    s.push_str("    if (!navigator.requestMIDIAccess) return false;\n");
    s.push_str("    try {\n");
    s.push_str("      const access = await navigator.requestMIDIAccess();\n");
    s.push_str("      for (const input of access.inputs.values()) {\n");
    s.push_str("        input.onmidimessage = (e) => this._onMessage(e);\n");
    s.push_str("      }\n");
    s.push_str("      return true;\n");
    s.push_str("    } catch(e) { return false; }\n");
    s.push_str("  }\n\n");

    s.push_str("  _onMessage(e) {\n");
    s.push_str("    const [status, cc, val] = e.data;\n");
    s.push_str("    const ch = status & 0x0F;\n");
    s.push_str(&format!(
        "    if (ch !== {}) return;\n", channel
    ));
    s.push_str("    const type = status & 0xF0;\n");
    s.push_str("    if (type === 0xB0) { // CC\n");
    s.push_str("      this._values[`cc${cc}`] = val / 127;\n");
    s.push_str("    } else if (type === 0x90) { // Note On\n");
    s.push_str("      this._values.note = cc;\n");
    s.push_str("      this._values.velocity = val / 127;\n");
    s.push_str("    } else if (type === 0x80) { // Note Off\n");
    s.push_str("      this._values.velocity = 0;\n");
    s.push_str("    }\n");
    s.push_str("  }\n\n");

    s.push_str("  get(name) { return this._values[name] || 0; }\n");
    s.push_str("  getAll() { return {...this._values}; }\n");
    s.push_str("}\n");

    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_midi_adapter() {
        let js = generate_midi_adapter(1);
        assert!(js.contains("class GameMidiAdapter"));
        assert!(js.contains("requestMIDIAccess"));
        assert!(js.contains("_channel = 1"));
    }
}
