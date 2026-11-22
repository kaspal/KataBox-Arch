//! Decision guides (`content/decide/<id>.toml`) and the glossary (`content/glossary.toml`).
//!
//! Decision guides: the id matches the file, numbers are unique, every answer goes to an
//! existing question or ends at a component kind (never both), every question is reachable
//! from the start, there are no loops (so every path ends at an entry), and all text is in
//! both languages.
//!
//! The glossary: ids are unique, terms and definitions are in both languages, `see` names a
//! kind, and every term is used somewhere in the rest of the content (so the list stays the
//! vocabulary the app really speaks).

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use ka_core::decide::{DecideFlow, Glossary, GlossaryTerm};

use crate::{Issue, TODO_MARKER, issue};

#[derive(Debug, Clone)]
pub struct FlowFile {
    pub path: String,
    pub flow: DecideFlow,
    pub toml: String,
}

pub fn load_decide(root: &Path) -> Result<Vec<FlowFile>, Vec<Issue>> {
    let base = root.join("decide");
    if !base.exists() {
        return Ok(Vec::new());
    }
    let mut names: Vec<String> = std::fs::read_dir(&base)
        .map_err(|e| vec![issue("decide", e.to_string())])?
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".toml"))
        .collect();
    names.sort();
    let mut out = Vec::new();
    let mut issues = Vec::new();
    for name in names {
        let path = format!("decide/{name}");
        match std::fs::read_to_string(base.join(&name)) {
            Ok(toml) => match toml::from_str::<DecideFlow>(&toml) {
                Ok(flow) => out.push(FlowFile { path, flow, toml }),
                Err(e) => issues.push(issue(path, e.to_string())),
            },
            Err(e) => issues.push(issue(path, e.to_string())),
        }
    }
    if issues.is_empty() {
        Ok(out)
    } else {
        Err(issues)
    }
}

pub fn validate_decide(flows: &[FlowFile]) -> Vec<Issue> {
    let mut issues = Vec::new();
    let mut numbers = BTreeSet::new();
    for f in flows {
        let d = &f.flow;
        let mut bad = |m: String| issues.push(issue(&f.path, m));
        if f.path != format!("decide/{}.toml", d.id) {
            bad(format!("id `{}` doesn't match the file name", d.id));
        }
        if !numbers.insert(d.number) {
            bad(format!("number {} is used twice", d.number));
        }
        if f.toml.contains(TODO_MARKER) {
            bad(format!("unfinished: contains {TODO_MARKER}"));
        }
        for (name, en, es) in [
            ("title", &d.title, &d.title_es),
            ("intro", &d.intro, &d.intro_es),
        ] {
            if en.trim().is_empty() || es.trim().is_empty() {
                bad(format!("`{name}` needs English and Spanish"));
            }
        }
        let by_id: BTreeMap<&str, usize> = d
            .questions
            .iter()
            .enumerate()
            .map(|(i, q)| (q.id.as_str(), i))
            .collect();
        if by_id.len() != d.questions.len() {
            bad("two questions share an id".into());
        }
        if !by_id.contains_key(d.start.as_str()) {
            bad(format!("start `{}` isn't a question", d.start));
            continue;
        }
        for q in &d.questions {
            if q.text.trim().is_empty() || q.text_es.trim().is_empty() {
                bad(format!("question `{}` needs English and Spanish", q.id));
            }
            if q.options.len() < 2 {
                bad(format!("question `{}` needs at least two answers", q.id));
            }
            for o in &q.options {
                if [&o.label, &o.label_es, &o.why, &o.why_es]
                    .iter()
                    .any(|s| s.trim().is_empty())
                {
                    bad(format!(
                        "an answer of `{}` needs its label and why in English and Spanish",
                        q.id
                    ));
                }
                match (&o.next, &o.result) {
                    (Some(n), None) if !by_id.contains_key(n.as_str()) => {
                        bad(format!("`{}` leads to `{n}`, which isn't a question", q.id))
                    }
                    (Some(_), Some(_)) | (None, None) => bad(format!(
                        "an answer of `{}` needs exactly one of `next` or `result`",
                        q.id
                    )),
                    _ => {}
                }
            }
        }
        // Reachable from the start, without loops: a depth-first walk with the path on a stack.
        let mut reached = BTreeSet::new();
        let mut looped = false;
        fn walk<'a>(
            q: &'a str,
            flow: &'a DecideFlow,
            by_id: &BTreeMap<&str, usize>,
            path: &mut Vec<&'a str>,
            reached: &mut BTreeSet<&'a str>,
            looped: &mut bool,
        ) {
            if path.contains(&q) {
                *looped = true;
                return;
            }
            reached.insert(q);
            path.push(q);
            if let Some(&i) = by_id.get(q) {
                for o in &flow.questions[i].options {
                    if let Some(n) = &o.next {
                        walk(n, flow, by_id, path, reached, looped);
                    }
                }
            }
            path.pop();
        }
        walk(
            &d.start,
            d,
            &by_id,
            &mut Vec::new(),
            &mut reached,
            &mut looped,
        );
        if looped {
            bad("the questions loop: every path must end at a component".into());
        }
        for q in &d.questions {
            if !reached.contains(q.id.as_str()) {
                bad(format!(
                    "question `{}` can't be reached from the start",
                    q.id
                ));
            }
        }
    }
    issues
}

pub fn load_glossary(root: &Path) -> Result<Vec<GlossaryTerm>, Vec<Issue>> {
    let path = root.join("glossary.toml");
    if !path.exists() {
        return Ok(Vec::new());
    }
    let text =
        std::fs::read_to_string(&path).map_err(|e| vec![issue("glossary.toml", e.to_string())])?;
    let g: Glossary =
        toml::from_str(&text).map_err(|e| vec![issue("glossary.toml", e.to_string())])?;
    Ok(g.terms)
}

/// `corpus` is every other content file's text (lower-cased by the caller or not; the check
/// lower-cases both).
pub fn validate_glossary(terms: &[GlossaryTerm], corpus: &str) -> Vec<Issue> {
    let mut issues = Vec::new();
    let corpus = corpus.to_lowercase();
    let mut ids = BTreeSet::new();
    for t in terms {
        let mut bad = |m: String| issues.push(issue("glossary.toml", format!("`{}`: {m}", t.id)));
        if !ids.insert(t.id.as_str()) {
            bad("the id is used twice".into());
        }
        if [&t.term, &t.term_es, &t.definition, &t.definition_es]
            .iter()
            .any(|s| s.trim().is_empty())
        {
            bad("needs the term and its definition in English and Spanish".into());
        }
        if !corpus.contains(&t.term.to_lowercase()) {
            bad(format!(
                "\"{}\" isn't used anywhere else in the content",
                t.term
            ));
        }
    }
    issues
}

/// Every content file except the glossary itself, concatenated (for the "is it used" check).
pub fn corpus(root: &Path) -> String {
    let mut out = String::new();
    fn visit(dir: &Path, out: &mut String) {
        let Ok(rd) = std::fs::read_dir(dir) else {
            return;
        };
        let mut entries: Vec<_> = rd.filter_map(|e| e.ok()).map(|e| e.path()).collect();
        entries.sort();
        for p in entries {
            if p.is_dir() {
                visit(&p, out);
            } else if p.file_name().is_some_and(|n| n != "glossary.toml")
                && p.extension().is_some_and(|x| x == "md" || x == "toml")
                && let Ok(t) = std::fs::read_to_string(&p)
            {
                out.push_str(&t);
                out.push('\n');
            }
        }
    }
    visit(root, &mut out);
    out
}
