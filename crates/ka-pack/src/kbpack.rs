//! Problem packs (M6.4, ADR 0011): `.kbpack` files anyone can make and share. A pack is a
//! zip of `manifest.toml` plus `problems/<id>/` folders in the ordinary problem layout.
//! Ported from KataBox's `kb-pack` (ADR 0038 there), without code files or assignments.
//!
//! Opening one runs the threat model's checks in order: T1–T4 in [`archive`] (names,
//! types, sizes, extensions); then here T5 (every file listed with its SHA-256, nothing
//! more or less), T6 (the manifest's fields; the pack id may not be `core` or a built-in
//! problem id), T7 (no raw HTML or script URLs in the Markdown, as defence in depth: the
//! app renders pack text with links inert and images as their alt text anyway), T8 (the
//! same structural validation as built-in content; [`judge_pack`] then runs the designs),
//! and T9 (caps on tests, request types, durations, cards and traffic, so a pack can't
//! make the simulator run for minutes). Pack problems are stored as `<pack>--<id>`.

pub mod archive;

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use ka_core::Registry;
use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::problems::ProblemDir;

pub const FORMAT: &str = "katabox-arch-pack/v1";
pub const MANIFEST: &str = "manifest.toml";

/// T9: what one pack problem may ask of the simulator.
pub const MAX_TESTS: usize = 10;
pub const MAX_REQUEST_TYPES: usize = 12;
pub const MAX_DURATION_S: f64 = 120.0;
pub const MAX_NODES: usize = 150;
pub const MAX_PEAK_RPS: f64 = 1e6;
pub const MAX_PROBLEMS: usize = 50;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub format: String,
    pub id: String,
    pub name: String,
    pub author: String,
    pub version: String,
    /// An SPDX licence expression, as stated by the author.
    pub licence: String,
    /// The oldest app version that can open the pack.
    pub min_app: String,
    /// Every other file in the pack, with its SHA-256 (hex).
    pub files: BTreeMap<String, String>,
}

pub use ka_app::packs::{Version, valid_pack_id};

fn plain_text(s: &str, max: usize) -> bool {
    !s.trim().is_empty() && s.chars().count() <= max && !s.chars().any(char::is_control)
}

/// T7: Markdown that could reach a browser engine as markup or a script URL.
pub fn markdown_problem(text: &str) -> Option<&'static str> {
    let lower = text.to_ascii_lowercase();
    for (needle, why) in [
        ("<script", "a <script> tag"),
        ("<iframe", "an <iframe> tag"),
        ("<object", "an <object> tag"),
        ("<embed", "an <embed> tag"),
        ("<style", "a <style> tag"),
        ("javascript:", "a javascript: URL"),
        ("vbscript:", "a vbscript: URL"),
        ("data:", "a data: URL"),
    ] {
        if lower.contains(needle) {
            return Some(why);
        }
    }
    // An event handler inside a tag: `<img onerror=…>`.
    let b = lower.as_bytes();
    let mut in_tag = false;
    for (i, &c) in b.iter().enumerate() {
        match c {
            b'<' => in_tag = true,
            b'>' => in_tag = false,
            b'o' if in_tag
                && b.get(i + 1) == Some(&b'n')
                && i > 0
                && b[i - 1].is_ascii_whitespace() =>
            {
                let rest = &lower[i + 2..];
                let name_len = rest.bytes().take_while(u8::is_ascii_lowercase).count();
                if name_len > 0 && rest[name_len..].trim_start().starts_with('=') {
                    return Some("an event handler attribute");
                }
            }
            _ => {}
        }
    }
    None
}

/// T9: a problem's demands on the simulator, or why they are too much.
pub fn simulation_caps(p: &ProblemDir) -> Vec<String> {
    let d = &p.def;
    let mut out = Vec::new();
    if d.tests.len() > MAX_TESTS {
        out.push(format!("at most {MAX_TESTS} tests"));
    }
    if d.request_types.len() > MAX_REQUEST_TYPES {
        out.push(format!("at most {MAX_REQUEST_TYPES} request types"));
    }
    if d.tests.iter().any(|t| t.duration_s > MAX_DURATION_S) {
        out.push(format!("tests last at most {MAX_DURATION_S} s"));
    }
    if d.nfr.peak_rps > MAX_PEAK_RPS {
        out.push("peak_rps is at most 1,000,000".into());
    }
    if p.reference.nodes.len() > MAX_NODES || p.naive.nodes.len() > MAX_NODES {
        out.push(format!("designs have at most {MAX_NODES} cards"));
    }
    out
}

/// What a pack holds, read and checked: ready to show before importing.
#[derive(Debug)]
pub struct OpenedPack {
    pub manifest: Manifest,
    /// The staging folder it was extracted to.
    pub dir: PathBuf,
    /// Its problems, ids still without the pack prefix.
    pub problems: Vec<ProblemDir>,
}

/// Every check after extraction (T5–T9, structure only). `builtin` are the built-in
/// problem ids (a pack id may not equal one); `app_version` is this app's version.
pub fn check_extracted(
    dir: &Path,
    files: &archive::Extracted,
    builtin: &BTreeSet<String>,
    app_version: &str,
    registry: &Registry,
) -> Result<OpenedPack, Vec<String>> {
    let mut errs = Vec::new();
    let manifest: Manifest = match fs::read_to_string(dir.join(MANIFEST))
        .map_err(|e| e.to_string())
        .and_then(|s| toml::from_str(&s).map_err(|e| e.to_string()))
    {
        Ok(m) => m,
        Err(e) => return Err(vec![format!("{MANIFEST}: {e}")]),
    };
    // T6: the manifest itself.
    if manifest.format != FORMAT {
        errs.push(format!("format must be {FORMAT:?}"));
    }
    if !valid_pack_id(&manifest.id) {
        errs.push("id: 3-20 lowercase letters and digits, starting with a letter".into());
    }
    if manifest.id == "core" || builtin.contains(&manifest.id) {
        errs.push(format!("id {:?} is taken by built-in content", manifest.id));
    }
    if !plain_text(&manifest.name, 80) || !plain_text(&manifest.author, 80) {
        errs.push("name and author: 1-80 characters, no control characters".into());
    }
    if !plain_text(&manifest.licence, 64)
        || !manifest
            .licence
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b" .+-()".contains(&b))
    {
        errs.push("licence: an SPDX expression such as CC-BY-SA-4.0".into());
    }
    if Version::parse(&manifest.version).is_none() {
        errs.push("version: MAJOR.MINOR.PATCH".into());
    }
    match (
        Version::parse(&manifest.min_app),
        Version::parse(app_version),
    ) {
        (Some(min), Some(app)) if min > app => errs.push(format!(
            "needs KataBox Arch {} or newer (this is {app_version})",
            manifest.min_app
        )),
        (None, _) => errs.push("min_app: MAJOR.MINOR.PATCH".into()),
        _ => {}
    }
    // T5: exactly the listed files, each with its hash.
    for (path, sha) in files {
        if path == MANIFEST {
            continue;
        }
        match manifest.files.get(path) {
            None => errs.push(format!("{path}: not listed in the manifest")),
            Some(want) if !want.eq_ignore_ascii_case(sha) => {
                errs.push(format!("{path}: SHA-256 does not match the manifest"))
            }
            Some(_) => {}
        }
    }
    for path in manifest.files.keys() {
        if !files.contains_key(path) {
            errs.push(format!("{path}: listed in the manifest but missing"));
        }
    }
    // Layout, then T7.
    for path in files.keys() {
        if path != MANIFEST && !path.starts_with("problems/") {
            errs.push(format!("{path}: packs hold problems/ only"));
        }
        if path.ends_with(".md")
            && let Some(why) = fs::read_to_string(dir.join(path))
                .ok()
                .as_deref()
                .and_then(markdown_problem)
        {
            errs.push(format!("{path}: {why} is not allowed in pack text"));
        }
    }
    if !errs.is_empty() {
        return Err(errs);
    }
    // T8: the same loading (strict `.kbarch` parser, size caps) and structural validation
    // as built-in content; T9: the simulation caps.
    let problems = crate::problems::load_problems(dir, registry)
        .map_err(|issues| issues.iter().map(ToString::to_string).collect::<Vec<_>>())?;
    if problems.is_empty() {
        errs.push("the pack has no problems".into());
    }
    if problems.len() > MAX_PROBLEMS {
        errs.push(format!("a pack holds at most {MAX_PROBLEMS} problems"));
    }
    errs.extend(
        crate::problems::validate_problems(&problems, registry)
            .iter()
            .map(ToString::to_string),
    );
    for p in &problems {
        errs.extend(
            simulation_caps(p)
                .into_iter()
                .map(|e| format!("{}: {e}", p.dir)),
        );
        let full = format!("{}--{}", manifest.id, p.def.id);
        if full.len() > 64 {
            errs.push(format!("{}: {full} is longer than 64 characters", p.dir));
        }
    }
    if errs.is_empty() {
        Ok(OpenedPack {
            manifest,
            dir: dir.to_path_buf(),
            problems,
        })
    } else {
        Err(errs)
    }
}

/// T8, the engine half: every problem's reference is Accepted on every test and its naive
/// design fails a hidden one (the rule built-in content follows).
pub fn judge_pack(pack: &OpenedPack, registry: &Registry) -> Vec<String> {
    crate::run::run_checks_quiet(&pack.problems, registry)
        .iter()
        .map(ToString::to_string)
        .collect()
}

/// Write a pack from problem folders: the manifest with every hash, then a zip with
/// sorted entries and a fixed timestamp, so the same input gives the same bytes.
pub fn write_pack(problem_dirs: &[PathBuf], head: &PackHead, out: &Path) -> Result<String, String> {
    let bytes = pack_bytes(problem_dirs, head)?;
    fs::write(out, &bytes).map_err(|e| e.to_string())?;
    Ok(archive::hex(&Sha256::digest(&bytes)))
}

/// A pack's own fields, as the author states them.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackHead {
    pub id: String,
    pub name: String,
    pub author: String,
    pub version: String,
    pub licence: String,
    pub min_app: String,
}

/// The `.kbpack` bytes for problem folders (deterministic).
pub fn pack_bytes(problem_dirs: &[PathBuf], head: &PackHead) -> Result<Vec<u8>, String> {
    let mut files: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    for dir in problem_dirs {
        let id = dir
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or("bad folder name")?
            .to_string();
        for e in fs::read_dir(dir).map_err(|e| e.to_string())? {
            let p = e.map_err(|e| e.to_string())?.path();
            if !p.is_file() {
                continue;
            }
            let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("");
            if !archive::ALLOWED_EXTENSIONS.contains(&ext) {
                continue;
            }
            let name = p
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or("bad file name")?;
            files.insert(
                format!("problems/{id}/{name}"),
                fs::read(&p).map_err(|e| e.to_string())?,
            );
        }
    }
    pack_bytes_from(&files, head)
}

/// The `.kbpack` bytes for files already in memory (`problems/<id>/<file>` → bytes).
pub fn pack_bytes_from(
    files: &BTreeMap<String, Vec<u8>>,
    head: &PackHead,
) -> Result<Vec<u8>, String> {
    // Serialised by the TOML crate, so any text the author typed is escaped correctly.
    #[derive(serde::Serialize)]
    struct Out<'a> {
        format: &'a str,
        id: &'a str,
        name: &'a str,
        author: &'a str,
        version: &'a str,
        licence: &'a str,
        min_app: &'a str,
        files: BTreeMap<&'a str, String>,
    }
    let manifest = toml::to_string(&Out {
        format: FORMAT,
        id: &head.id,
        name: &head.name,
        author: &head.author,
        version: &head.version,
        licence: &head.licence,
        min_app: &head.min_app,
        files: files
            .iter()
            .map(|(p, d)| (p.as_str(), archive::hex(&Sha256::digest(d))))
            .collect(),
    })
    .map_err(|e| e.to_string())?;
    let mut buf = std::io::Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut buf);
        let opts = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .last_modified_time(zip::DateTime::default())
            .unix_permissions(0o644);
        zip.start_file(MANIFEST, opts).map_err(|e| e.to_string())?;
        zip.write_all(manifest.as_bytes())
            .map_err(|e| e.to_string())?;
        for (path, data) in files {
            zip.start_file(path.as_str(), opts)
                .map_err(|e| e.to_string())?;
            zip.write_all(data).map_err(|e| e.to_string())?;
        }
        zip.finish().map_err(|e| e.to_string())?;
    }
    Ok(buf.into_inner())
}

/// The pack's own `content.db` data: its problems only, stored as `<pack>--<id>`.
pub fn pack_content(
    pack: &OpenedPack,
    registry: &Registry,
) -> Result<ka_store::ContentData, Vec<String>> {
    let mut problems = Vec::new();
    for p in &pack.problems {
        let mut row = p
            .row(registry)
            .map_err(|e| vec![format!("{}: {e}", p.dir)])?;
        let full = format!("{}--{}", pack.manifest.id, p.def.id);
        let mut def = p.def.clone();
        def.id.clone_from(&full);
        row.def = serde_json::to_string(&def).map_err(|e| vec![e.to_string()])?;
        row.id = full;
        problems.push(row);
    }
    Ok(ka_store::ContentData {
        meta: vec![
            ("pack_id".into(), pack.manifest.id.clone()),
            ("pack_version".into(), pack.manifest.version.clone()),
            ("schema".into(), ka_store::CONTENT_SCHEMA.to_string()),
        ],
        problems,
        ..Default::default()
    })
}

/// The pack checks, plugged into the app (`ka_app::App::set_pack_opener`).
pub struct Opener;

impl ka_app::packs::PackOpener for Opener {
    fn open(
        &self,
        archive_path: &Path,
        staging: &Path,
        builtin: &BTreeSet<String>,
        app_version: &str,
        registry: &Registry,
    ) -> Result<ka_app::packs::OpenedPackData, Vec<String>> {
        let sha256 = archive::sha256_file(archive_path)
            .map_err(|e| vec![format!("cannot read the pack: {e}")])?;
        let files = archive::extract(archive_path, staging).map_err(|r| vec![r.to_string()])?;
        let pack = check_extracted(staging, &files, builtin, app_version, registry)?;
        let issues = judge_pack(&pack, registry);
        if !issues.is_empty() {
            return Err(issues);
        }
        let data = pack_content(&pack, registry)?;
        tokio::runtime::Builder::new_current_thread()
            .build()
            .map_err(|e| vec![e.to_string()])?
            .block_on(ka_store::write_content(&staging.join("content.db"), &data))
            .map_err(|e| vec![e.to_string()])?;
        Ok(ka_app::packs::OpenedPackData {
            id: pack.manifest.id.clone(),
            name: pack.manifest.name.clone(),
            author: pack.manifest.author.clone(),
            version: pack.manifest.version.clone(),
            licence: pack.manifest.licence.clone(),
            sha256,
            problems: data
                .problems
                .iter()
                .map(|r| ka_app::packs::PackProblem {
                    id: r.id.clone(),
                    title_en: r.title_en.clone(),
                    title_es: r.title_es.clone(),
                    difficulty: r.difficulty.clone(),
                })
                .collect(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pack_ids_and_markdown() {
        assert!(valid_pack_id("graphs101"));
        for bad in [
            "ab",
            "two-sum",
            "Graphs",
            "1abc",
            "a_b_c",
            "abcdefghijklmnopqrstu",
        ] {
            assert!(!valid_pack_id(bad), "{bad}");
        }
        assert_eq!(markdown_problem("A cache: `ttl_s = 60`. One = 1."), None);
        assert!(markdown_problem("<img src=x onerror=alert(1)>").is_some());
        assert!(markdown_problem("[x](javascript:alert(1))").is_some());
        assert!(markdown_problem("![x](data:image/png;base64,AAAA)").is_some());
        assert!(markdown_problem("<SCRIPT>bad()</SCRIPT>").is_some());
    }
}
