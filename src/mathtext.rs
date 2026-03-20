//! Basic scientific text rendering using Unicode.
//!
//! Provides Greek letter substitution, superscript/subscript digits,
//! and scientific notation formatting for axis labels and annotations.
//!
//! # Example
//!
//! ```
//! use ratatui_plt::mathtext::render_mathtext;
//!
//! assert_eq!(render_mathtext(r"\alpha"), "α");
//! assert_eq!(render_mathtext("x^2"), "x²");
//! assert_eq!(render_mathtext("x_0"), "x₀");
//! assert_eq!(render_mathtext(r"\rho_0 \times 10^3"), "ρ₀ × 10³");
//! ```

use std::collections::HashMap;

/// Render a mathtext string using Unicode substitutions.
///
/// Supports:
/// - Greek letters: `\alpha`, `\beta`, `\gamma`, etc.
/// - Superscript digits: `^0` through `^9`, `^-`, `^+`
/// - Subscript digits: `_0` through `_9`, `_-`, `_+`
/// - Operators: `\times`, `\pm`, `\approx`, `\leq`, `\geq`, `\neq`, `\infty`
pub fn render_mathtext(input: &str) -> String {
    let greek = greek_map();
    let operators = operator_map();

    let mut result = String::with_capacity(input.len());
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '\\' {
            // Try to match a command
            let _start = i;
            i += 1;
            let mut cmd = String::new();
            while i < chars.len() && chars[i].is_ascii_alphabetic() {
                cmd.push(chars[i]);
                i += 1;
            }
            if let Some(&replacement) = greek.get(cmd.as_str()) {
                result.push_str(replacement);
            } else if let Some(&replacement) = operators.get(cmd.as_str()) {
                result.push_str(replacement);
            } else {
                // Unknown command, emit as-is
                result.push('\\');
                result.push_str(&cmd);
            }
        } else if chars[i] == '^' && i + 1 < chars.len() {
            i += 1;
            if chars[i] == '{' {
                // Grouped superscript: ^{...}
                i += 1;
                while i < chars.len() && chars[i] != '}' {
                    if let Some(sup) = to_superscript(chars[i]) {
                        result.push(sup);
                    } else {
                        result.push(chars[i]);
                    }
                    i += 1;
                }
                if i < chars.len() {
                    i += 1; // skip '}'
                }
            } else {
                // Single character superscript
                if let Some(sup) = to_superscript(chars[i]) {
                    result.push(sup);
                } else {
                    result.push('^');
                    result.push(chars[i]);
                }
                i += 1;
            }
        } else if chars[i] == '_' && i + 1 < chars.len() {
            i += 1;
            if chars[i] == '{' {
                i += 1;
                while i < chars.len() && chars[i] != '}' {
                    if let Some(sub) = to_subscript(chars[i]) {
                        result.push(sub);
                    } else {
                        result.push(chars[i]);
                    }
                    i += 1;
                }
                if i < chars.len() {
                    i += 1;
                }
            } else {
                if let Some(sub) = to_subscript(chars[i]) {
                    result.push(sub);
                } else {
                    result.push('_');
                    result.push(chars[i]);
                }
                i += 1;
            }
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }

    result
}

/// Convert a digit character to its Unicode superscript form.
pub fn to_superscript(c: char) -> Option<char> {
    match c {
        '0' => Some('\u{2070}'),
        '1' => Some('\u{00B9}'),
        '2' => Some('\u{00B2}'),
        '3' => Some('\u{00B3}'),
        '4' => Some('\u{2074}'),
        '5' => Some('\u{2075}'),
        '6' => Some('\u{2076}'),
        '7' => Some('\u{2077}'),
        '8' => Some('\u{2078}'),
        '9' => Some('\u{2079}'),
        '-' => Some('\u{207B}'),
        '+' => Some('\u{207A}'),
        'n' => Some('\u{207F}'),
        'i' => Some('\u{2071}'),
        _ => None,
    }
}

/// Convert a digit character to its Unicode subscript form.
pub fn to_subscript(c: char) -> Option<char> {
    match c {
        '0' => Some('\u{2080}'),
        '1' => Some('\u{2081}'),
        '2' => Some('\u{2082}'),
        '3' => Some('\u{2083}'),
        '4' => Some('\u{2084}'),
        '5' => Some('\u{2085}'),
        '6' => Some('\u{2086}'),
        '7' => Some('\u{2087}'),
        '8' => Some('\u{2088}'),
        '9' => Some('\u{2089}'),
        '-' => Some('\u{208B}'),
        '+' => Some('\u{208A}'),
        _ => None,
    }
}

/// Format a number in scientific notation using Unicode superscripts.
///
/// # Example
///
/// ```
/// use ratatui_plt::mathtext::scientific_notation;
///
/// assert_eq!(scientific_notation(1.5e-3), "1.5×10⁻³");
/// assert_eq!(scientific_notation(2.0e6), "2×10⁶");
/// ```
pub fn scientific_notation(value: f64) -> String {
    if value == 0.0 {
        return "0".to_string();
    }
    let exp = value.abs().log10().floor() as i64;
    let mantissa = value / 10.0_f64.powi(exp as i32);

    let mantissa_str = if (mantissa - mantissa.round()).abs() < 1e-10 {
        format!("{}", mantissa.round() as i64)
    } else {
        format!("{:.1}", mantissa)
    };

    let exp_str: String = format!("{}", exp)
        .chars()
        .map(|c| to_superscript(c).unwrap_or(c))
        .collect();

    format!("{}×10{}", mantissa_str, exp_str)
}

fn greek_map() -> HashMap<&'static str, &'static str> {
    let mut m = HashMap::new();
    m.insert("alpha", "α");
    m.insert("beta", "β");
    m.insert("gamma", "γ");
    m.insert("delta", "δ");
    m.insert("epsilon", "ε");
    m.insert("zeta", "ζ");
    m.insert("eta", "η");
    m.insert("theta", "θ");
    m.insert("iota", "ι");
    m.insert("kappa", "κ");
    m.insert("lambda", "λ");
    m.insert("mu", "μ");
    m.insert("nu", "ν");
    m.insert("xi", "ξ");
    m.insert("pi", "π");
    m.insert("rho", "ρ");
    m.insert("sigma", "σ");
    m.insert("tau", "τ");
    m.insert("upsilon", "υ");
    m.insert("phi", "φ");
    m.insert("chi", "χ");
    m.insert("psi", "ψ");
    m.insert("omega", "ω");
    m.insert("Gamma", "Γ");
    m.insert("Delta", "Δ");
    m.insert("Theta", "Θ");
    m.insert("Lambda", "Λ");
    m.insert("Xi", "Ξ");
    m.insert("Pi", "Π");
    m.insert("Sigma", "Σ");
    m.insert("Phi", "Φ");
    m.insert("Psi", "Ψ");
    m.insert("Omega", "Ω");
    m
}

fn operator_map() -> HashMap<&'static str, &'static str> {
    let mut m = HashMap::new();
    m.insert("times", "×");
    m.insert("pm", "±");
    m.insert("mp", "∓");
    m.insert("approx", "≈");
    m.insert("leq", "≤");
    m.insert("geq", "≥");
    m.insert("neq", "≠");
    m.insert("infty", "∞");
    m.insert("nabla", "∇");
    m.insert("partial", "∂");
    m.insert("sum", "Σ");
    m.insert("prod", "Π");
    m.insert("int", "∫");
    m.insert("sqrt", "√");
    m.insert("deg", "°");
    m.insert("cdot", "·");
    m.insert("ldots", "…");
    m.insert("hbar", "ℏ");
    m
}
