use crate::codegen::CompileOutput;

/// Generate a React wrapper component for the web component.
pub(super) fn generate_react(output: &CompileOutput, tag_name: &str) -> String {
    let pascal = to_pascal_case(tag_name);
    let mut s = String::new();

    s.push_str("import { useRef, useEffect } from 'react';\n\n");
    s.push_str("// Import the web component module in your app entry:\n");
    s.push_str("// import './component.js';\n\n");

    // Props interface
    s.push_str(&format!("export function {}({{\n", pascal));
    s.push_str("  width = '100%',\n");
    s.push_str("  height = '100%',\n");
    for f in &output.data_fields {
        s.push_str(&format!("  {} = 0.5,\n", f));
    }
    s.push_str("}) {\n");
    s.push_str("  const ref = useRef(null);\n\n");

    // Effects for data fields
    if !output.data_fields.is_empty() {
        s.push_str("  useEffect(() => {\n");
        s.push_str("    if (!ref.current) return;\n");
        for f in &output.data_fields {
            s.push_str(&format!("    ref.current.{} = {};\n", f, f));
        }
        s.push_str("  }, [");
        let deps: Vec<&str> = output.data_fields.iter().map(|f| f.as_str()).collect();
        s.push_str(&deps.join(", "));
        s.push_str("]);\n\n");
    }

    // Render
    s.push_str(&format!("  return <{} ref={{ref}} style={{{{ width, height }}}} />;\n", tag_name));
    s.push_str("}\n");
    s
}

/// Generate a Vue SFC wrapper for the web component.
pub(super) fn generate_vue(output: &CompileOutput, tag_name: &str) -> String {
    let mut s = String::new();

    // Template
    s.push_str("<template>\n");
    s.push_str(&format!("  <{} ref=\"comp\" :style=\"{{ width, height }}\" />\n", tag_name));
    s.push_str("</template>\n\n");

    // Script
    s.push_str("<script setup>\n");
    s.push_str("import { ref, onMounted, watch } from 'vue';\n\n");

    s.push_str("const props = defineProps({\n");
    s.push_str("  width: { type: String, default: '100%' },\n");
    s.push_str("  height: { type: String, default: '100%' },\n");
    for f in &output.data_fields {
        s.push_str(&format!("  {}: {{ type: Number, default: 0.5 }},\n", f));
    }
    s.push_str("});\n\n");

    s.push_str("const comp = ref(null);\n\n");

    if !output.data_fields.is_empty() {
        s.push_str("watch(\n");
        s.push_str("  () => [");
        let fields: Vec<String> = output.data_fields.iter().map(|f| format!("props.{}", f)).collect();
        s.push_str(&fields.join(", "));
        s.push_str("],\n");
        s.push_str("  () => {\n");
        s.push_str("    if (!comp.value) return;\n");
        for f in &output.data_fields {
            s.push_str(&format!("    comp.value.{} = props.{};\n", f, f));
        }
        s.push_str("  },\n");
        s.push_str(");\n\n");
    }

    s.push_str("onMounted(() => {\n");
    s.push_str("  // Load web component — adjust path as needed\n");
    s.push_str("  const script = document.createElement('script');\n");
    s.push_str("  script.type = 'module';\n");
    s.push_str("  script.src = './component.js';\n");
    s.push_str("  document.head.appendChild(script);\n");
    if !output.data_fields.is_empty() {
        s.push_str("  // Set initial data values\n");
        s.push_str("  if (comp.value) {\n");
        for f in &output.data_fields {
            s.push_str(&format!("    comp.value.{} = props.{};\n", f, f));
        }
        s.push_str("  }\n");
    }
    s.push_str("});\n");
    s.push_str("</script>\n");
    s
}

/// Generate a CSS-only fallback approximation.
pub(super) fn generate_css(output: &CompileOutput, tag_name: &str) -> String {
    let mut s = String::new();

    s.push_str(&format!("/* CSS fallback for <{}> — static approximation */\n\n", tag_name));
    s.push_str(&format!("{} {{\n", tag_name));
    s.push_str("  display: block;\n");
    s.push_str("  width: 200px;\n");
    s.push_str("  height: 200px;\n");
    s.push_str("  background: #0A0A0A;\n");
    s.push_str("  border-radius: 8px;\n");
    s.push_str("  position: relative;\n");
    s.push_str("  overflow: hidden;\n");
    s.push_str("}\n\n");

    s.push_str(&format!("{}::before {{\n", tag_name));
    s.push_str("  content: '';\n");
    s.push_str("  position: absolute;\n");
    s.push_str("  inset: 0;\n");

    // Approximate based on render mode
    match &output.render_mode {
        crate::codegen::RenderMode::Flat => {
            s.push_str("  background: radial-gradient(circle at center,\n");
            s.push_str("    rgba(255, 255, 255, 0.15) 0%,\n");
            s.push_str("    rgba(255, 255, 255, 0.05) 40%,\n");
            s.push_str("    transparent 70%);\n");
        }
        crate::codegen::RenderMode::Raymarch { .. } => {
            s.push_str("  background: radial-gradient(circle at 40% 40%,\n");
            s.push_str("    rgba(255, 255, 255, 0.2) 0%,\n");
            s.push_str("    rgba(255, 255, 255, 0.05) 30%,\n");
            s.push_str("    transparent 60%);\n");
        }
    }

    s.push_str("  border-radius: inherit;\n");
    s.push_str("}\n");
    s
}

fn to_pascal_case(tag_name: &str) -> String {
    tag_name
        .split('-')
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => {
                    let upper: String = first.to_uppercase().collect();
                    upper + chars.as_str()
                }
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_pascal_case_works() {
        assert_eq!(to_pascal_case("dashboard-gauge"), "DashboardGauge");
        assert_eq!(to_pascal_case("my-cool-widget"), "MyCoolWidget");
        assert_eq!(to_pascal_case("simple"), "Simple");
    }
}
