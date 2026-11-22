//! Estimation drills (M5.2): `content/estimation/<problem-id>.toml`. Each drill states its
//! inputs and a formula; the answer is always computed from them (never typed by hand), and
//! a user's answer is judged against a ratio band around it.
//!
//! Formulas use numbers (`1e6`, `2.5`, `1_000`), input names, `+ - * / ^` and parentheses:
//! nothing else, so a formula can't do anything but arithmetic.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum DrillKind {
    Qps,
    Storage,
    Bandwidth,
    Servers,
    /// How much a part must hold at once: a queue's depth, connections, calls in flight.
    Capacity,
}

/// One drill as written in the content file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DrillDef {
    pub id: String,
    pub kind: DrillKind,
    /// The question; `{name}` shows an input's value.
    pub prompt: String,
    pub prompt_es: String,
    pub inputs: BTreeMap<String, f64>,
    pub formula: String,
    /// The answer's unit (`NUMBER_UNITS`).
    pub unit: String,
    /// The accepted band: answer / (1 + t) ..= answer × (1 + t).
    #[serde(default = "default_tolerance")]
    pub tolerance: f64,
    /// The worked solution; `{name}` shows an input, `{answer}` the result.
    pub solution: String,
    pub solution_es: String,
}

fn default_tolerance() -> f64 {
    0.5
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DrillFile {
    pub drill: Vec<DrillDef>,
}

/// A problem's drills as `content.db` stores them, with every answer computed at build time.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DrillSet {
    pub problem_id: String,
    pub drills: Vec<DrillDef>,
    /// `answers[i]` belongs to `drills[i]`.
    pub answers: Vec<f64>,
}

/// A drill as the UI gets it before answering: no answer, no formula, no solution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Drill {
    pub problem_id: String,
    pub id: String,
    pub kind: DrillKind,
    pub prompt: String,
    pub prompt_es: String,
    pub inputs: BTreeMap<String, f64>,
    pub unit: String,
}

/// How close an answer was.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum Closeness {
    /// Within the drill's band.
    Close,
    /// Within ×10 but outside the band.
    TooHigh,
    TooLow,
    /// More than ten times off: probably a units slip.
    WayOff,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct EstimateCheck {
    pub ok: bool,
    pub closeness: Closeness,
    pub answer: f64,
    /// The worked solution as written, with `{name}` and `{answer}` placeholders: the UI
    /// fills them with `inputs` and `answer` in the reader's number format.
    pub solution: String,
    pub solution_es: String,
    pub inputs: BTreeMap<String, f64>,
}

pub fn judge(answer: f64, given: f64, tolerance: f64) -> Closeness {
    if !(given.is_finite() && given > 0.0 && answer > 0.0) {
        return Closeness::WayOff;
    }
    let ratio = given / answer;
    let band = 1.0 + tolerance.max(0.0);
    if ratio <= band && ratio >= 1.0 / band {
        Closeness::Close
    } else if !(0.1..=10.0).contains(&ratio) {
        Closeness::WayOff
    } else if ratio > 1.0 {
        Closeness::TooHigh
    } else {
        Closeness::TooLow
    }
}

// --- The evaluator ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Num(f64),
    Name(String),
    Op(char),
    Open,
    Close,
}

fn lex(s: &str) -> Result<Vec<Tok>, String> {
    let c: Vec<char> = s.chars().collect();
    let mut i = 0;
    let mut out = Vec::new();
    while i < c.len() {
        let ch = c[i];
        if ch.is_whitespace() {
            i += 1;
        } else if ch.is_ascii_digit() || ch == '.' {
            let start = i;
            while i < c.len() && (c[i].is_ascii_digit() || c[i] == '.' || c[i] == '_') {
                i += 1;
            }
            if i < c.len() && (c[i] == 'e' || c[i] == 'E') {
                i += 1;
                if i < c.len() && (c[i] == '+' || c[i] == '-') {
                    i += 1;
                }
                while i < c.len() && c[i].is_ascii_digit() {
                    i += 1;
                }
            }
            let text: String = c[start..i].iter().filter(|x| **x != '_').collect();
            out.push(Tok::Num(
                text.parse().map_err(|_| format!("bad number `{text}`"))?,
            ));
        } else if ch.is_ascii_alphabetic() || ch == '_' {
            let start = i;
            while i < c.len() && (c[i].is_ascii_alphanumeric() || c[i] == '_') {
                i += 1;
            }
            out.push(Tok::Name(c[start..i].iter().collect()));
        } else if "+-*/^".contains(ch) {
            out.push(Tok::Op(ch));
            i += 1;
        } else if ch == '(' {
            out.push(Tok::Open);
            i += 1;
        } else if ch == ')' {
            out.push(Tok::Close);
            i += 1;
        } else {
            return Err(format!("unexpected `{ch}`"));
        }
    }
    Ok(out)
}

/// The parsed formula.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Num(f64),
    Var(String),
    Neg(Box<Expr>),
    Bin(char, Box<Expr>, Box<Expr>),
}

struct Parser {
    toks: Vec<Tok>,
    at: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.at)
    }
    fn next(&mut self) -> Option<Tok> {
        let t = self.toks.get(self.at).cloned();
        self.at += 1;
        t
    }
    // sum := product (('+'|'-') product)*
    fn sum(&mut self) -> Result<Expr, String> {
        let mut e = self.product()?;
        while let Some(Tok::Op(op @ ('+' | '-'))) = self.peek().cloned() {
            self.at += 1;
            e = Expr::Bin(op, Box::new(e), Box::new(self.product()?));
        }
        Ok(e)
    }
    // product := unary (('*'|'/') unary)*
    fn product(&mut self) -> Result<Expr, String> {
        let mut e = self.unary()?;
        while let Some(Tok::Op(op @ ('*' | '/'))) = self.peek().cloned() {
            self.at += 1;
            e = Expr::Bin(op, Box::new(e), Box::new(self.unary()?));
        }
        Ok(e)
    }
    // unary := '-' unary | power   (so -2^2 is -(2^2))
    fn unary(&mut self) -> Result<Expr, String> {
        if let Some(Tok::Op('-')) = self.peek() {
            self.at += 1;
            return Ok(Expr::Neg(Box::new(self.unary()?)));
        }
        self.power()
    }
    // power := atom ('^' unary)?   (right-associative)
    fn power(&mut self) -> Result<Expr, String> {
        let base = self.atom()?;
        if let Some(Tok::Op('^')) = self.peek() {
            self.at += 1;
            return Ok(Expr::Bin('^', Box::new(base), Box::new(self.unary()?)));
        }
        Ok(base)
    }
    fn atom(&mut self) -> Result<Expr, String> {
        match self.next() {
            Some(Tok::Num(n)) => Ok(Expr::Num(n)),
            Some(Tok::Name(n)) => Ok(Expr::Var(n)),
            Some(Tok::Open) => {
                let e = self.sum()?;
                match self.next() {
                    Some(Tok::Close) => Ok(e),
                    _ => Err("missing `)`".into()),
                }
            }
            Some(t) => Err(format!("unexpected {t:?}")),
            None => Err("the formula ends too early".into()),
        }
    }
}

pub fn parse(formula: &str) -> Result<Expr, String> {
    let mut p = Parser {
        toks: lex(formula)?,
        at: 0,
    };
    let e = p.sum()?;
    if p.at != p.toks.len() {
        return Err(format!("unexpected {:?}", p.toks[p.at]));
    }
    Ok(e)
}

/// The names a formula uses, in first-use order.
pub fn names(e: &Expr, out: &mut Vec<String>) {
    match e {
        Expr::Num(_) => {}
        Expr::Var(v) => {
            if !out.contains(v) {
                out.push(v.clone());
            }
        }
        Expr::Neg(x) => names(x, out),
        Expr::Bin(_, a, b) => {
            names(a, out);
            names(b, out);
        }
    }
}

pub fn eval(e: &Expr, vars: &BTreeMap<String, f64>) -> Result<f64, String> {
    let v = match e {
        Expr::Num(n) => *n,
        Expr::Var(name) => *vars.get(name).ok_or_else(|| format!("unknown `{name}`"))?,
        Expr::Neg(x) => -eval(x, vars)?,
        Expr::Bin(op, a, b) => {
            let (a, b) = (eval(a, vars)?, eval(b, vars)?);
            match op {
                '+' => a + b,
                '-' => a - b,
                '*' => a * b,
                '/' => a / b,
                '^' => crate::detmath::powf(a, b),
                _ => unreachable!("the lexer only makes these operators"),
            }
        }
    };
    Ok(v)
}

/// The drill's answer, from its inputs and formula.
pub fn answer(d: &DrillDef) -> Result<f64, String> {
    let v = eval(&parse(&d.formula)?, &d.inputs)?;
    if v.is_finite() && v > 0.0 {
        Ok(v)
    } else {
        Err(format!("the formula gives {v}"))
    }
}

/// `{name}` replaced by the input's value (and `{answer}` by the answer), numbers written
/// with thousands separators in the reader's language.
pub fn fill(text: &str, inputs: &BTreeMap<String, f64>, answer: Option<f64>, es: bool) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let Some(close) = rest[open..].find('}') else {
            out.push_str(&rest[open..]);
            return out;
        };
        let name = &rest[open + 1..open + close];
        let value = if name == "answer" {
            answer
        } else {
            inputs.get(name).copied()
        };
        match value {
            Some(v) => out.push_str(&format_number(v, es)),
            None => out.push_str(&rest[open..=open + close]),
        }
        rest = &rest[open + close + 1..];
    }
    out.push_str(rest);
    out
}

/// A number for prose: up to three significant decimals, grouped thousands.
pub fn format_number(v: f64, es: bool) -> String {
    let rounded = if v.abs() >= 100.0 {
        v.round()
    } else if v.abs() >= 1.0 {
        (v * 10.0).round() / 10.0
    } else {
        (v * 1000.0).round() / 1000.0
    };
    let text = format!("{rounded}");
    let (int, frac) = match text.split_once('.') {
        Some((i, f)) => (i.to_owned(), Some(f.to_owned())),
        None => (text, None),
    };
    let (sign, digits) = match int.strip_prefix('-') {
        Some(d) => ("-", d.to_owned()),
        None => ("", int),
    };
    let sep = if es { '\u{a0}' } else { ',' };
    let mut grouped = String::new();
    // Spanish style leaves four-digit numbers ungrouped.
    let group = !(es && digits.len() <= 4);
    for (i, ch) in digits.chars().enumerate() {
        if group && i > 0 && (digits.len() - i) % 3 == 0 {
            grouped.push(sep);
        }
        grouped.push(ch);
    }
    let point = if es { ',' } else { '.' };
    match frac {
        Some(f) => format!("{sign}{grouped}{point}{f}"),
        None => format!("{sign}{grouped}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vars(pairs: &[(&str, f64)]) -> BTreeMap<String, f64> {
        pairs.iter().map(|(k, v)| ((*k).to_owned(), *v)).collect()
    }

    #[test]
    fn formulas_follow_the_usual_precedence() {
        let v = vars(&[("users", 1e8), ("per", 10.0)]);
        let e = |s: &str| eval(&parse(s).unwrap(), &v).unwrap();
        assert_eq!(e("users * per / 86_400"), 1e8 * 10.0 / 86_400.0);
        assert_eq!(e("2 + 3 * 4"), 14.0);
        assert_eq!(e("(2 + 3) * 4"), 20.0);
        // `^` uses the deterministic `powf`: close, not bit-exact.
        assert!((e("2 ^ 3 ^ 2") - 512.0).abs() < 1e-9);
        assert!((e("-2 ^ 2") + 4.0).abs() < 1e-9);
        assert_eq!(e("1.5e3 - 500"), 1000.0);
    }

    #[test]
    fn bad_formulas_are_refused() {
        for bad in ["2 +", "(1", "1)", "a b", "max(1, 2)", "1 % 2", "users;"] {
            assert!(parse(bad).is_err(), "{bad}");
        }
        assert!(eval(&parse("nope * 2").unwrap(), &vars(&[])).is_err());
        let mut names_out = Vec::new();
        names(&parse("a * (b + a) / c").unwrap(), &mut names_out);
        assert_eq!(names_out, ["a", "b", "c"]);
    }

    #[test]
    fn answers_are_judged_in_a_ratio_band() {
        assert_eq!(judge(100.0, 140.0, 0.5), Closeness::Close);
        assert_eq!(judge(100.0, 67.0, 0.5), Closeness::Close);
        assert_eq!(judge(100.0, 160.0, 0.5), Closeness::TooHigh);
        assert_eq!(judge(100.0, 60.0, 0.5), Closeness::TooLow);
        assert_eq!(judge(100.0, 1001.0, 0.5), Closeness::WayOff);
        assert_eq!(judge(100.0, 9.0, 0.5), Closeness::WayOff);
        assert_eq!(judge(100.0, -5.0, 0.5), Closeness::WayOff);
        assert_eq!(judge(100.0, f64::NAN, 0.5), Closeness::WayOff);
    }

    #[test]
    fn prose_shows_inputs_and_the_answer() {
        let v = vars(&[("users", 12_000_000.0), ("share", 0.25)]);
        assert_eq!(
            fill(
                "{users} users, {share} of them: {answer}/s {nope}",
                &v,
                Some(3472.2),
                false
            ),
            "12,000,000 users, 0.25 of them: 3,472/s {nope}"
        );
        assert_eq!(
            fill("{users} y {answer}", &v, Some(1234.5), true),
            "12\u{a0}000\u{a0}000 y 1235"
        );
        assert_eq!(format_number(2.55, true), "2,6");
    }
}
