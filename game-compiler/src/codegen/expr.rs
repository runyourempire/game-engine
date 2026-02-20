use super::WgslGen;
use crate::ast::*;
use crate::error::{GameError, Result};

impl WgslGen {
    // ── Expression compilation (WGSL) ──────────────────────────────────

    pub(super) fn compile_arg(&self, args: &[Arg], index: usize, default: &str) -> Result<String> {
        match args.get(index) {
            Some(Arg::Positional(expr)) => self.compile_expr(expr),
            Some(Arg::Named { value, .. }) => self.compile_expr(value),
            None => Ok(default.to_string()),
        }
    }

    pub(super) fn compile_named_arg(&self, args: &[Arg], name: &str, default: &str) -> Result<String> {
        for arg in args {
            if let Arg::Named { name: n, value } = arg {
                if n == name {
                    return self.compile_expr(value);
                }
            }
        }
        Ok(default.to_string())
    }

    /// Resolve tint color from args. Supports named colors (gold, red, etc.) or vec3f.
    pub(super) fn compile_tint_color(&self, args: &[Arg]) -> Result<String> {
        if let Some(arg) = args.first() {
            match arg {
                Arg::Positional(expr) => self.compile_expr(expr),
                Arg::Named { value, .. } => self.compile_expr(value),
            }
        } else {
            Ok("vec3f(1.0)".to_string())
        }
    }

    fn compile_expr(&self, expr: &Expr) -> Result<String> {
        match expr {
            Expr::Number(n) => {
                if n.fract() == 0.0 {
                    Ok(format!("{n:.1}"))
                } else {
                    Ok(format!("{n}"))
                }
            }
            Expr::String(s) => Ok(format!("\"{s}\"")),
            Expr::Ident(name) => Ok(compile_ident(name)),
            Expr::FieldAccess { object, field } => {
                let obj = self.compile_expr(object)?;
                Ok(format!("{obj}.{field}"))
            }
            Expr::BinaryOp { left, op, right } => {
                let l = self.compile_expr(left)?;
                let r = self.compile_expr(right)?;
                let op_str = match op {
                    BinOp::Add => "+",
                    BinOp::Sub => "-",
                    BinOp::Mul => "*",
                    BinOp::Div => "/",
                    BinOp::Gt => ">",
                    BinOp::Lt => "<",
                };
                Ok(format!("({l} {op_str} {r})"))
            }
            Expr::Negate(inner) => {
                let s = self.compile_expr(inner)?;
                Ok(format!("(-{s})"))
            }
            Expr::Call(call) => self.compile_call(call),
            Expr::Array(elements) => {
                let compiled: Result<Vec<String>> =
                    elements.iter().map(|e| self.compile_expr(e)).collect();
                let compiled = compiled?;
                match compiled.len() {
                    2 => Ok(format!("vec2f({})", compiled.join(", "))),
                    3 => Ok(format!("vec3f({})", compiled.join(", "))),
                    4 => Ok(format!("vec4f({})", compiled.join(", "))),
                    _ => Ok(format!("array({})", compiled.join(", "))),
                }
            }
            Expr::Ternary { condition, if_true, if_false } => {
                let cond = self.compile_expr(condition)?;
                let t = self.compile_expr(if_true)?;
                let f = self.compile_expr(if_false)?;
                Ok(format!("select({f}, {t}, {cond})"))
            }
        }
    }

    fn compile_call(&self, call: &FnCall) -> Result<String> {
        let args: Result<Vec<String>> = call.args.iter()
            .map(|a| match a {
                Arg::Positional(e) => self.compile_expr(e),
                Arg::Named { value, .. } => self.compile_expr(value),
            })
            .collect();
        let args = args?;

        match call.name.as_str() {
            "sin" | "cos" | "tan" | "asin" | "acos" | "atan" | "sqrt" | "abs" | "sign"
            | "floor" | "ceil" | "round" | "fract" | "length" | "normalize" | "exp" | "log"
            | "log2" | "saturate" => Ok(format!("{}({})", call.name, args.join(", "))),
            "pow" | "min" | "max" | "dot" | "cross" | "distance" | "atan2" | "step" => {
                Ok(format!("{}({})", call.name, args.join(", ")))
            }
            "mix" | "lerp" => Ok(format!("mix({})", args.join(", "))),
            "clamp" => Ok(format!("clamp({})", args.join(", "))),
            "smoothstep" => Ok(format!("smoothstep({})", args.join(", "))),
            "mod" => Ok(format!("(({}) % ({}))", args[0], args[1])),
            _ => Err(GameError::unknown_function(&call.name)),
        }
    }
}

// ── Free functions ─────────────────────────────────────────────────────

fn compile_ident(name: &str) -> String {
    match name {
        "time" => "time".to_string(),
        "p" => "p".to_string(),
        "uv" => "input.uv".to_string(),
        "height" => "height".to_string(),
        "pi" => "3.14159265359".to_string(),
        "tau" => "6.28318530718".to_string(),
        "e" => "2.71828182846".to_string(),
        "phi" => "1.61803398875".to_string(),
        // Colors
        "black" => "vec3f(0.0)".to_string(),
        "white" => "vec3f(1.0)".to_string(),
        "red" => "vec3f(1.0, 0.0, 0.0)".to_string(),
        "green" => "vec3f(0.0, 1.0, 0.0)".to_string(),
        "blue" => "vec3f(0.0, 0.0, 1.0)".to_string(),
        "gold" => "vec3f(0.831, 0.686, 0.216)".to_string(),
        "midnight" => "vec3f(0.0, 0.0, 0.1)".to_string(),
        "obsidian" => "vec3f(0.04, 0.04, 0.06)".to_string(),
        "ember" => "vec3f(0.8, 0.2, 0.05)".to_string(),
        "cyan" => "vec3f(0.0, 1.0, 1.0)".to_string(),
        "ivory" => "vec3f(1.0, 0.97, 0.92)".to_string(),
        "frost" => "vec3f(0.85, 0.92, 1.0)".to_string(),
        "orange" => "vec3f(1.0, 0.5, 0.0)".to_string(),
        "deep_blue" => "vec3f(0.0, 0.02, 0.15)".to_string(),
        _ => name.to_string(),
    }
}

/// Compile an AST expression to JavaScript (for runtime modulation).
pub fn compile_expr_js(expr: &Expr) -> String {
    match expr {
        Expr::Number(n) => {
            if n.fract() == 0.0 {
                format!("{n:.1}")
            } else {
                format!("{n}")
            }
        }
        Expr::Ident(name) => match name.as_str() {
            "time" => "time".to_string(),
            _ => name.clone(),
        },
        Expr::FieldAccess { object, field } => {
            let obj = compile_expr_js(object);
            match (obj.as_str(), field.as_str()) {
                ("audio", "bass") => "audioBass".to_string(),
                ("audio", "mid") => "audioMid".to_string(),
                ("audio", "treble") => "audioTreble".to_string(),
                ("audio", "energy") => "audioEnergy".to_string(),
                ("audio", "beat") => "audioBeat".to_string(),
                ("mouse", "x") => "mouseX".to_string(),
                ("mouse", "y") => "mouseY".to_string(),
                ("data", f) => format!("data_{f}"),
                _ => format!("{obj}_{field}"),
            }
        }
        Expr::BinaryOp { left, op, right } => {
            let l = compile_expr_js(left);
            let r = compile_expr_js(right);
            let op_str = match op {
                BinOp::Add => "+",
                BinOp::Sub => "-",
                BinOp::Mul => "*",
                BinOp::Div => "/",
                BinOp::Gt => ">",
                BinOp::Lt => "<",
            };
            format!("({l} {op_str} {r})")
        }
        Expr::Negate(inner) => {
            let s = compile_expr_js(inner);
            format!("(-{s})")
        }
        Expr::Call(call) => {
            let args: Vec<String> = call.args.iter()
                .map(|a| match a {
                    Arg::Positional(e) => compile_expr_js(e),
                    Arg::Named { value, .. } => compile_expr_js(value),
                })
                .collect();
            format!("Math.{}({})", call.name, args.join(", "))
        }
        Expr::String(s) => format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"")),
        Expr::Array(elements) => {
            let compiled: Vec<String> = elements.iter().map(compile_expr_js).collect();
            format!("[{}]", compiled.join(", "))
        }
        Expr::Ternary { condition, if_true, if_false } => {
            let cond = compile_expr_js(condition);
            let t = compile_expr_js(if_true);
            let f = compile_expr_js(if_false);
            format!("({cond} ? {t} : {f})")
        }
    }
}
