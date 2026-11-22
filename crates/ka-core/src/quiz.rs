//! Quick-check quizzes (M5.5): three questions at the end of every Guide entry and pattern,
//! written as `[[quiz]]` in `entry.toml` / `pattern.toml`. Each has one correct option and
//! an explanation. Missed questions return in a spaced review queue (ADR 0010).

use serde::{Deserialize, Serialize};

/// One question as written in content.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(deny_unknown_fields)]
pub struct QuizQuestion {
    pub q: String,
    pub q_es: String,
    /// Three or four options; `answer` is the index of the right one.
    pub options: Vec<String>,
    pub options_es: Vec<String>,
    pub answer: u32,
    /// Why the answer is right (shown after answering).
    pub why: String,
    pub why_es: String,
}

/// Questions per Guide entry and per pattern.
pub const QUIZ_LEN: usize = 3;

/// A question as the UI gets it before answering: no answer, no explanation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct QuizItem {
    /// `guide:<kind>:<n>` or `pattern:<id>:<n>` (n = 1..=3).
    pub id: String,
    pub q: String,
    pub q_es: String,
    pub options: Vec<String>,
    pub options_es: Vec<String>,
}

/// What answering returns.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct QuizResult {
    pub correct: bool,
    pub answer: u32,
    pub why: String,
    pub why_es: String,
    /// When the question comes back for review (`YYYY-MM-DD`); None if it won't.
    pub due_on: Option<String>,
}

/// The id of question `n` (1-based) of a page (`guide:<kind>` or `pattern:<id>`).
pub fn question_id(page: &str, n: usize) -> String {
    format!("{page}:{n}")
}

/// The page and 1-based number of a question id.
pub fn split_question_id(id: &str) -> Option<(&str, usize)> {
    let (page, n) = id.rsplit_once(':')?;
    let n: usize = n.parse().ok()?;
    let (kind, target) = page.split_once(':')?;
    ((kind == "guide" || kind == "pattern") && !target.is_empty() && (1..=QUIZ_LEN).contains(&n))
        .then_some((page, n))
}

/// Problems with a page's questions (empty when they are sound).
pub fn check(questions: &[QuizQuestion]) -> Vec<String> {
    let mut out = Vec::new();
    if questions.len() != QUIZ_LEN {
        out.push(format!(
            "needs exactly {QUIZ_LEN} quiz questions, has {}",
            questions.len()
        ));
    }
    for (i, q) in questions.iter().enumerate() {
        let n = i + 1;
        if q.q.trim().is_empty() || q.q_es.trim().is_empty() {
            out.push(format!("question {n}: empty text"));
        }
        if q.why.trim().is_empty() || q.why_es.trim().is_empty() {
            out.push(format!("question {n}: no explanation"));
        }
        if !(3..=4).contains(&q.options.len()) {
            out.push(format!("question {n}: needs 3 or 4 options"));
        }
        if q.options.len() != q.options_es.len() {
            out.push(format!("question {n}: options differ between languages"));
        }
        if q.answer as usize >= q.options.len() {
            out.push(format!("question {n}: the answer isn't among the options"));
        }
        for (lang, opts) in [("en", &q.options), ("es", &q.options_es)] {
            let mut seen: Vec<String> = Vec::new();
            for o in opts {
                let norm = o.trim().to_lowercase();
                if norm.is_empty() {
                    out.push(format!("question {n}: an empty option ({lang})"));
                } else if seen.contains(&norm) {
                    out.push(format!("question {n}: a repeated option ({lang})"));
                }
                seen.push(norm);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn q(options: &[&str], answer: u32) -> QuizQuestion {
        QuizQuestion {
            q: "Q?".into(),
            q_es: "¿P?".into(),
            options: options.iter().map(|s| (*s).to_owned()).collect(),
            options_es: options.iter().map(|s| format!("{s} es")).collect(),
            answer,
            why: "Because.".into(),
            why_es: "Porque.".into(),
        }
    }

    #[test]
    fn questions_are_checked() {
        let good = q(&["a", "b", "c"], 2);
        assert!(check(&[good.clone(), good.clone(), good.clone()]).is_empty());
        let issues = check(&[q(&["a", "b", "c"], 3), q(&["a", "A", "c"], 0)]);
        let text = issues.join("\n");
        assert!(text.contains("exactly 3"), "{text}");
        assert!(text.contains("isn't among the options"), "{text}");
        assert!(text.contains("repeated option (en)"), "{text}");
        let two = q(&["a", "b"], 0);
        assert!(
            check(&[two.clone(), two.clone(), two])
                .join("")
                .contains("3 or 4")
        );
    }

    #[test]
    fn question_ids_round_trip() {
        assert_eq!(question_id("guide:cache", 2), "guide:cache:2");
        assert_eq!(split_question_id("guide:cache:2"), Some(("guide:cache", 2)));
        assert_eq!(
            split_question_id("pattern:cache-aside:3"),
            Some(("pattern:cache-aside", 3))
        );
        assert_eq!(split_question_id("guide:cache:4"), None);
        assert_eq!(split_question_id("problem:x:1"), None);
        assert_eq!(split_question_id("nope"), None);
    }
}
