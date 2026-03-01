//! Temporal operator codegen — emits JavaScript runtime classes for
//! delay (>>), smooth (<>), trigger (!!), and range (..) operators.

use crate::ast::{Duration, Expr, Param, TemporalOp};

/// Check whether any params in a cinematic use temporal operators.
pub fn any_param_uses_temporal(params: &[Param]) -> bool {
    params.iter().any(|p| !p.temporal_ops.is_empty())
}

/// Convert a Duration to seconds (float).
fn duration_to_seconds(d: &Duration) -> f64 {
    match d {
        Duration::Seconds(v) => *v,
        Duration::Millis(v) => *v / 1000.0,
        Duration::Bars(v) => *v as f64 * 2.0, // default 120 BPM = 2s per bar
    }
}

/// Convert an Expr to a JS literal string.
fn expr_to_js(e: &Expr) -> String {
    match e {
        Expr::Number(v) => format!("{v}"),
        Expr::Ident(name) => name.clone(),
        Expr::DottedIdent { object, field } => format!("{object}.{field}"),
        _ => "0".into(),
    }
}

/// Emit the JS class for RingBuffer (delay operator >>).
pub fn ring_buffer_class() -> &'static str {
    r#"class GameRingBuffer {
  constructor(durationSec, fps = 60) {
    this._size = Math.max(1, Math.ceil(durationSec * fps));
    this._buffer = new Float32Array(this._size);
    this._head = 0;
  }
  push(value) {
    this._buffer[this._head] = value;
    this._head = (this._head + 1) % this._size;
  }
  delayed() {
    return this._buffer[this._head];
  }
}
"#
}

/// Emit the JS class for EMA filter (smooth operator <>).
pub fn ema_filter_class() -> &'static str {
    r#"class GameEMAFilter {
  constructor(durationSec, fps = 60) {
    const samples = Math.max(1, durationSec * fps);
    this._alpha = 2.0 / (samples + 1.0);
    this._value = null;
  }
  update(value) {
    if (this._value === null) { this._value = value; return value; }
    this._value += this._alpha * (value - this._value);
    return this._value;
  }
}
"#
}

/// Emit the JS class for edge detector (trigger operator !!).
pub fn edge_detector_class() -> &'static str {
    r#"class GameEdgeDetector {
  constructor(decaySec, fps = 60) {
    this._decayRate = 1.0 / Math.max(1, decaySec * fps);
    this._prev = 0;
    this._envelope = 0;
  }
  update(value) {
    const delta = value - this._prev;
    this._prev = value;
    if (delta > 0.01) { this._envelope = 1.0; }
    else { this._envelope = Math.max(0, this._envelope - this._decayRate); }
    return this._envelope;
  }
}
"#
}

/// Generate JS temporal processing initialization code for a set of params.
///
/// Returns (init_code, update_code) where:
/// - init_code: class definitions + per-param processor creation
/// - update_code: per-frame processing calls
pub fn generate_temporal_js(params: &[Param]) -> (String, String) {
    let mut needs_ring = false;
    let mut needs_ema = false;
    let mut needs_edge = false;

    // Scan for which classes are needed
    for param in params {
        for op in &param.temporal_ops {
            match op {
                TemporalOp::Delay(_) => needs_ring = true,
                TemporalOp::Smooth(_) => needs_ema = true,
                TemporalOp::Trigger(_) => needs_edge = true,
                TemporalOp::Range(_, _) => {} // Inline clamp, no class needed
            }
        }
    }

    let mut init = String::new();

    // Emit needed classes
    if needs_ring {
        init.push_str(ring_buffer_class());
    }
    if needs_ema {
        init.push_str(ema_filter_class());
    }
    if needs_edge {
        init.push_str(edge_detector_class());
    }

    // Per-param processor instances
    let mut update = String::new();

    for param in params {
        if param.temporal_ops.is_empty() {
            continue;
        }

        let pname = &param.name;

        for (i, op) in param.temporal_ops.iter().enumerate() {
            let suffix = if param.temporal_ops.len() > 1 {
                format!("_{i}")
            } else {
                String::new()
            };

            match op {
                TemporalOp::Delay(dur) => {
                    let secs = duration_to_seconds(dur);
                    init.push_str(&format!(
                        "const _delay_{pname}{suffix} = new GameRingBuffer({secs});\n"
                    ));
                    update.push_str(&format!(
                        "_delay_{pname}{suffix}.push(_val_{pname}); _val_{pname} = _delay_{pname}{suffix}.delayed();\n"
                    ));
                }
                TemporalOp::Smooth(dur) => {
                    let secs = duration_to_seconds(dur);
                    init.push_str(&format!(
                        "const _smooth_{pname}{suffix} = new GameEMAFilter({secs});\n"
                    ));
                    update.push_str(&format!(
                        "_val_{pname} = _smooth_{pname}{suffix}.update(_val_{pname});\n"
                    ));
                }
                TemporalOp::Trigger(dur) => {
                    let secs = duration_to_seconds(dur);
                    init.push_str(&format!(
                        "const _trigger_{pname}{suffix} = new GameEdgeDetector({secs});\n"
                    ));
                    update.push_str(&format!(
                        "_val_{pname} = _trigger_{pname}{suffix}.update(_val_{pname});\n"
                    ));
                }
                TemporalOp::Range(min_expr, max_expr) => {
                    let min_js = expr_to_js(min_expr);
                    let max_js = expr_to_js(max_expr);
                    update.push_str(&format!(
                        "_val_{pname} = Math.min(Math.max(_val_{pname}, {min_js}), {max_js});\n"
                    ));
                }
            }
        }
    }

    (init, update)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::*;

    fn param_with_ops(name: &str, ops: Vec<TemporalOp>) -> Param {
        Param {
            name: name.into(),
            value: Expr::Number(0.5),
            modulation: None,
            temporal_ops: ops,
        }
    }

    #[test]
    fn no_temporal_ops_empty_output() {
        let params = vec![Param {
            name: "x".into(),
            value: Expr::Number(1.0),
            modulation: None,
            temporal_ops: vec![],
        }];
        let (init, update) = generate_temporal_js(&params);
        assert!(init.is_empty());
        assert!(update.is_empty());
    }

    #[test]
    fn delay_emits_ring_buffer() {
        let params = vec![param_with_ops(
            "bass",
            vec![TemporalOp::Delay(Duration::Millis(200.0))],
        )];
        let (init, update) = generate_temporal_js(&params);
        assert!(init.contains("class GameRingBuffer"));
        assert!(init.contains("_delay_bass"));
        assert!(init.contains("0.2")); // 200ms = 0.2s
        assert!(update.contains("_delay_bass"));
    }

    #[test]
    fn smooth_emits_ema() {
        let params = vec![param_with_ops(
            "energy",
            vec![TemporalOp::Smooth(Duration::Millis(50.0))],
        )];
        let (init, update) = generate_temporal_js(&params);
        assert!(init.contains("class GameEMAFilter"));
        assert!(init.contains("_smooth_energy"));
        assert!(update.contains("_smooth_energy"));
    }

    #[test]
    fn trigger_emits_edge_detector() {
        let params = vec![param_with_ops(
            "beat",
            vec![TemporalOp::Trigger(Duration::Millis(300.0))],
        )];
        let (init, update) = generate_temporal_js(&params);
        assert!(init.contains("class GameEdgeDetector"));
        assert!(init.contains("_trigger_beat"));
        assert!(update.contains("_trigger_beat"));
    }

    #[test]
    fn range_emits_clamp() {
        let params = vec![param_with_ops(
            "vol",
            vec![TemporalOp::Range(
                Expr::Number(0.0),
                Expr::Number(1.0),
            )],
        )];
        let (init, update) = generate_temporal_js(&params);
        // Range doesn't need a class
        assert!(!init.contains("class"));
        assert!(update.contains("Math.min(Math.max(_val_vol, 0), 1)"));
    }

    #[test]
    fn chained_ops_apply_in_order() {
        let params = vec![param_with_ops(
            "bass",
            vec![
                TemporalOp::Smooth(Duration::Millis(50.0)),
                TemporalOp::Delay(Duration::Millis(200.0)),
                TemporalOp::Range(Expr::Number(0.0), Expr::Number(1.0)),
            ],
        )];
        let (init, update) = generate_temporal_js(&params);
        assert!(init.contains("class GameEMAFilter"));
        assert!(init.contains("class GameRingBuffer"));
        // Update should apply smooth, then delay, then clamp
        let smooth_pos = update.find("_smooth_bass").unwrap();
        let delay_pos = update.find("_delay_bass").unwrap();
        let clamp_pos = update.find("Math.min").unwrap();
        assert!(smooth_pos < delay_pos, "smooth before delay");
        assert!(delay_pos < clamp_pos, "delay before clamp");
    }

    #[test]
    fn any_param_detection() {
        let no_temporal = vec![Param {
            name: "x".into(),
            value: Expr::Number(1.0),
            modulation: None,
            temporal_ops: vec![],
        }];
        assert!(!any_param_uses_temporal(&no_temporal));

        let with_temporal = vec![param_with_ops(
            "y",
            vec![TemporalOp::Smooth(Duration::Seconds(0.1))],
        )];
        assert!(any_param_uses_temporal(&with_temporal));
    }

    #[test]
    fn duration_conversion() {
        assert!((duration_to_seconds(&Duration::Seconds(1.5)) - 1.5).abs() < f64::EPSILON);
        assert!((duration_to_seconds(&Duration::Millis(500.0)) - 0.5).abs() < f64::EPSILON);
        assert!((duration_to_seconds(&Duration::Bars(2)) - 4.0).abs() < f64::EPSILON);
    }
}
