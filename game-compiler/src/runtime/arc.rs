//! Arc timeline JS generation — easing functions, timeline data, and update logic.

use crate::codegen::CompiledMoment;

/// Shared easing function library (emitted once, used by both HTML and component).
const EASING_FUNCTIONS_JS: &str = r#"const ease = {
  linear: t => t,
  expo_in: t => t === 0 ? 0 : Math.pow(2, 10 * (t - 1)),
  expo_out: t => t === 1 ? 1 : 1 - Math.pow(2, -10 * t),
  cubic_in_out: t => t < 0.5 ? 4*t*t*t : 1 - Math.pow(-2*t + 2, 3) / 2,
  smooth: t => t * t * (3 - 2 * t),
  elastic: t => t === 0 || t === 1 ? t : -Math.pow(2, 10*t - 10) * Math.sin((t*10 - 10.75) * (2*Math.PI/3)),
  bounce: t => { const n=7.5625, d=2.75; if(t<1/d) return n*t*t; if(t<2/d) return n*(t-=1.5/d)*t+0.75; if(t<2.5/d) return n*(t-=2.25/d)*t+0.9375; return n*(t-=2.625/d)*t+0.984375; },
};"#;

/// Generate JS arc timeline code for the HTML runtime.
/// Emits: easing functions, timeline data, arcUpdate() function that modifies params[].base.
pub(super) fn generate_arc_js(moments: &[CompiledMoment]) -> String {
    if moments.is_empty() {
        return "function arcUpdate() {}".to_string();
    }

    let mut js = String::with_capacity(1024);

    // Easing library
    js.push_str(EASING_FUNCTIONS_JS);
    js.push('\n');

    // Timeline data: array of { t, transitions: [{ paramIdx, target, animated, ease, dur }] }
    js.push_str("const arcTimeline = [\n");
    for (i, m) in moments.iter().enumerate() {
        let name_str = m.name.as_deref().unwrap_or("");
        js.push_str(&format!(
            "  {{ t: {}, name: '{}', transitions: [",
            m.time_seconds, name_str
        ));
        for t in &m.transitions {
            let dur = match t.duration_secs {
                Some(d) => format!("{d}"),
                None => {
                    // Duration until next moment (or 1s if last)
                    let next_t = moments.get(i + 1).map(|m| m.time_seconds).unwrap_or(m.time_seconds + 1.0);
                    format!("{}", next_t - m.time_seconds)
                }
            };
            js.push_str(&format!(
                "{{ pi: {}, to: {}, anim: {}, ease: '{}', dur: {} }},",
                t.param_index, t.target_value, t.is_animated, t.easing, dur
            ));
        }
        js.push_str("] },\n");
    }
    js.push_str("];\n\n");

    // Track "from" values (snapshotted when a transition starts)
    js.push_str("const arcState = new Map();\n");

    // arcUpdate function — called each frame with current time
    js.push_str(r#"function arcUpdate(time) {
  for (let mi = 0; mi < arcTimeline.length; mi++) {
    const m = arcTimeline[mi];
    if (time < m.t) continue;
    for (const tr of m.transitions) {
      const key = `${mi}_${tr.pi}`;
      if (!tr.anim) {
        // Instant set — override base immediately
        params[tr.pi].base = tr.to;
        continue;
      }
      // Animated transition
      const elapsed = time - m.t;
      const progress = Math.min(elapsed / tr.dur, 1.0);
      // Snapshot "from" value on first encounter
      if (!arcState.has(key)) {
        arcState.set(key, params[tr.pi].base);
      }
      const from = arcState.get(key);
      const easeFn = ease[tr.ease] || ease.linear;
      const t = easeFn(progress);
      params[tr.pi].base = from + (tr.to - from) * t;
    }
  }
}
"#);

    js
}

/// Generate inline arc update code for Web Component _frame() method.
pub(super) fn generate_arc_component_js(moments: &[CompiledMoment]) -> String {
    if moments.is_empty() {
        return String::new();
    }

    let mut js = String::with_capacity(512);

    // Initialize arc data on first frame
    js.push_str("    if (!this._arcTimeline) {\n");
    js.push_str("      ");
    js.push_str(EASING_FUNCTIONS_JS);
    js.push('\n');
    js.push_str("      this._arcEase = ease;\n");
    js.push_str("      this._arcTimeline = [\n");

    for (i, m) in moments.iter().enumerate() {
        let name_str = m.name.as_deref().unwrap_or("");
        js.push_str(&format!(
            "        {{ t: {}, name: '{}', transitions: [",
            m.time_seconds, name_str
        ));
        for t in &m.transitions {
            let dur = match t.duration_secs {
                Some(d) => format!("{d}"),
                None => {
                    let next_t = moments.get(i + 1).map(|m| m.time_seconds).unwrap_or(m.time_seconds + 1.0);
                    format!("{}", next_t - m.time_seconds)
                }
            };
            js.push_str(&format!(
                "{{ pi: {}, to: {}, anim: {}, ease: '{}', dur: {} }},",
                t.param_index, t.target_value, t.is_animated, t.easing, dur
            ));
        }
        js.push_str("] },\n");
    }

    js.push_str("      ];\n");
    js.push_str("      this._arcState = new Map();\n");
    js.push_str("    }\n");

    // Inline arc update
    js.push_str(r#"    for (let mi = 0; mi < this._arcTimeline.length; mi++) {
      const m = this._arcTimeline[mi];
      if (time < m.t) continue;
      for (const tr of m.transitions) {
        const key = `${mi}_${tr.pi}`;
        if (!tr.anim) {
          this._arcBaseOverrides = this._arcBaseOverrides || {};
          this._arcBaseOverrides[tr.pi] = tr.to;
          continue;
        }
        const elapsed = time - m.t;
        const progress = Math.min(elapsed / tr.dur, 1.0);
        if (!this._arcState.has(key)) {
          this._arcState.set(key, this._arcBaseOverrides?.[tr.pi] ?? 0);
        }
        const from = this._arcState.get(key);
        const easeFn = this._arcEase[tr.ease] || this._arcEase.linear;
        const t = easeFn(progress);
        this._arcBaseOverrides = this._arcBaseOverrides || {};
        this._arcBaseOverrides[tr.pi] = from + (tr.to - from) * t;
      }
    }
"#);

    js
}
