//! Hostile packs (M6.4, ADR 0011's threat model): every fixture is built here, in the
//! test, and must be refused for the right reason; a well-formed pack round-trips.
//! T10 (versions) and T11 (where files are written) are the app's: `ka-app` tests them.
use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::path::{Path, PathBuf};

use ka_core::registry;
use ka_pack::kbpack::{self, PackHead, archive};

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn temp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("ka-pack-hostile-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

type Entry<'a> = (&'a str, Vec<u8>);

/// A zip of `entries` (deflate), in order.
fn zip_of(entries: &[Entry]) -> Vec<u8> {
    let mut buf = std::io::Cursor::new(Vec::new());
    let mut z = zip::ZipWriter::new(&mut buf);
    let opts = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    for (name, data) in entries {
        z.start_file(*name, opts).unwrap();
        z.write_all(data).unwrap();
    }
    z.finish().unwrap();
    buf.into_inner()
}

fn extract_err(bytes: &[u8], name: &str) -> String {
    let dir = temp(name);
    archive::extract_bytes(bytes, &dir.join("src"))
        .expect_err(name)
        .to_string()
}

/// A copy of a real problem folder, to edit. Bulk Email is cheap to simulate.
const FIXTURE: &str = "bulk-email";

fn problem_copy(name: &str) -> PathBuf {
    let dst = temp(name).join(FIXTURE);
    std::fs::create_dir_all(&dst).unwrap();
    for e in std::fs::read_dir(repo().join("content/problems").join(FIXTURE))
        .unwrap()
        .flatten()
    {
        std::fs::copy(e.path(), dst.join(e.file_name())).unwrap();
    }
    dst
}

fn head(id: &str, min_app: &str) -> PackHead {
    PackHead {
        id: id.into(),
        name: "Email drills".into(),
        author: "Ana".into(),
        version: "1.0.0".into(),
        licence: "CC-BY-SA-4.0".into(),
        min_app: min_app.into(),
    }
}

fn builtin(ids: &[&str]) -> BTreeSet<String> {
    ids.iter().map(|s| s.to_string()).collect()
}

/// Pack `problems`, then open the result like the app does.
fn pack_and_open(
    name: &str,
    problems: &[PathBuf],
    id: &str,
    built_in: &[&str],
) -> Result<kbpack::OpenedPack, Vec<String>> {
    let dir = temp(&format!("{name}-pack"));
    let out = dir.join("x.kbpack");
    kbpack::write_pack(problems, &head(id, "0.1.0"), &out).unwrap();
    let src = dir.join("src");
    let files = archive::extract(&out, &src).map_err(|r| vec![r.to_string()])?;
    kbpack::check_extracted(&src, &files, &builtin(built_in), "0.1.0", &registry())
}

#[test]
fn a_well_formed_pack_round_trips_is_judged_and_is_deterministic() {
    let p = problem_copy("good");
    let pack = pack_and_open("good", std::slice::from_ref(&p), "drills", &[FIXTURE])
        .unwrap_or_else(|e| panic!("{e:#?}"));
    assert_eq!(pack.manifest.id, "drills");
    assert_eq!(pack.problems.len(), 1);
    // T8, the engine half: the reference passes, the naive design fails a hidden test.
    assert_eq!(kbpack::judge_pack(&pack, &registry()), Vec::<String>::new());
    // The same input gives the same bytes.
    let d = temp("determinism");
    let a = kbpack::write_pack(
        std::slice::from_ref(&p),
        &head("drills", "0.1.0"),
        &d.join("a.kbpack"),
    )
    .unwrap();
    let b = kbpack::write_pack(
        std::slice::from_ref(&p),
        &head("drills", "0.1.0"),
        &d.join("b.kbpack"),
    )
    .unwrap();
    assert_eq!(a, b);
}

#[test]
fn t1_names_that_escape_or_confuse_are_refused() {
    let ok = b"x".to_vec();
    for (name, why) in [
        ("../evil.md", "`..`"),
        ("/abs.md", "absolute"),
        ("C:/drive.md", "only letters"),
        ("a\\b.md", "backslash"),
        ("problems/CON.md", "device"),
        ("problems/nul.txt.md", "device"),
        ("problems/a.md:stream", "only letters"),
        ("problems/.hidden.md", "dot"),
        ("a/b/c/d/e/f/g/h/i/deep.md", "too deep"),
    ] {
        let err = extract_err(
            &zip_of(&[(name, ok.clone())]),
            &format!("t1-{}", name.len()),
        );
        assert!(err.contains(why), "{name}: {err}");
    }
    let long = format!("problems/{}.md", "a".repeat(190));
    assert!(extract_err(&zip_of(&[(long.as_str(), ok.clone())]), "t1-long").contains("too long"));
    let dup = zip_of(&[("problems/A.md", ok.clone()), ("problems/a.md", ok)]);
    assert!(extract_err(&dup, "t1-case").contains("case"));
}

#[test]
fn t2_links_and_special_files_are_refused() {
    let mut buf = std::io::Cursor::new(Vec::new());
    {
        let mut z = zip::ZipWriter::new(&mut buf);
        z.add_symlink(
            "problems/link.md",
            "../../outside",
            zip::write::SimpleFileOptions::default(),
        )
        .unwrap();
        z.finish().unwrap();
    }
    assert!(extract_err(&buf.into_inner(), "t2-link").contains("links"));
    // A character device: the zip writer normalises modes, so set the central directory's
    // "made by Unix" and external attributes (mode << 16) by hand.
    let mut bytes = zip_of(&[("problems/dev.md", b"x".to_vec())]);
    let cd = bytes
        .windows(4)
        .position(|w| w == [0x50, 0x4b, 0x01, 0x02])
        .unwrap();
    bytes[cd + 5] = 3;
    bytes[cd + 38..cd + 42].copy_from_slice(&(0o020644u32 << 16).to_le_bytes());
    let err = extract_err(&bytes, "t2-dev");
    assert!(err.contains("plain files"), "{err}");
}

#[test]
fn t3_bombs_and_oversized_files_are_refused() {
    // 5 MB of zeros as a design: over the 1 MB cap (and far beyond 200:1).
    let bomb = zip_of(&[("problems/x/reference.kbarch", vec![0u8; 5 * 1024 * 1024])]);
    assert!(bomb.len() < 64 * 1024, "the fixture really is compressed");
    let err = extract_err(&bomb, "t3-bomb");
    assert!(
        err.contains("larger than") || err.contains("200:1"),
        "{err}"
    );
    // A 300 KB statement: over the 256 KB text cap.
    let big = zip_of(&[("problems/x/statement.md", vec![b'a'; 300 * 1024])]);
    assert!(extract_err(&big, "t3-text").contains("larger than 256 KB"));
    // 900 KB of one byte as a design: under the cap, but compressed far beyond 200:1.
    let ratio = zip_of(&[("problems/x/naive.kbarch", vec![b'0'; 900 * 1024])]);
    assert!(extract_err(&ratio, "t3-ratio").contains("200:1"));
    // Too many entries.
    let names: Vec<String> = (0..1001).map(|i| format!("problems/x/{i}.md")).collect();
    let many: Vec<Entry> = names.iter().map(|n| (n.as_str(), b"x".to_vec())).collect();
    assert!(extract_err(&zip_of(&many), "t3-many").contains("1000"));
    // An archive over 16 MB is refused before it is opened.
    let huge = vec![0u8; 17 * 1024 * 1024];
    assert!(extract_err(&huge, "t3-huge").contains("16 MB"));
}

#[test]
fn t4_only_text_file_types_are_accepted() {
    for name in [
        "problems/x/tool.exe",
        "problems/x/page.html",
        "problems/x/script.js",
        "problems/x/data.json",
        "problems/x/README",
        "problems/x/run.sh",
    ] {
        let err = extract_err(
            &zip_of(&[(name, b"x".to_vec())]),
            &format!("t4-{}", name.len()),
        );
        assert!(err.contains("file type not allowed"), "{name}: {err}");
    }
}

/// Extract the good pack, change the files on disk, re-hash them as if the zip had carried
/// them, and check.
fn tampered(name: &str, change: &dyn Fn(&Path)) -> String {
    let p = problem_copy(name);
    let dir = temp(&format!("{name}-pack"));
    let out = dir.join("x.kbpack");
    kbpack::write_pack(std::slice::from_ref(&p), &head("drills", "0.1.0"), &out).unwrap();
    let src = dir.join("src");
    archive::extract(&out, &src).unwrap();
    change(&src);
    let mut files = BTreeMap::new();
    for e in walk(&src) {
        let rel = e
            .strip_prefix(&src)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        let sha = archive::hex(&<sha2::Sha256 as sha2::Digest>::digest(
            std::fs::read(&e).unwrap(),
        ));
        files.insert(rel, sha);
    }
    kbpack::check_extracted(&src, &files, &BTreeSet::new(), "0.1.0", &registry())
        .unwrap_err()
        .join("\n")
}

fn walk(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for e in std::fs::read_dir(dir).unwrap().flatten() {
        let p = e.path();
        if p.is_dir() {
            out.extend(walk(&p))
        } else {
            out.push(p)
        }
    }
    out
}

#[test]
fn t5_hashes_must_match_the_manifest_exactly() {
    let flipped = tampered("t5-flip", &|src| {
        let f = src.join("problems/bulk-email/statement.md");
        let mut b = std::fs::read(&f).unwrap();
        b[0] ^= 1;
        std::fs::write(f, b).unwrap();
    });
    assert!(flipped.contains("SHA-256 does not match"), "{flipped}");
    let extra = tampered("t5-extra", &|src| {
        std::fs::write(src.join("problems/bulk-email/extra.md"), "x").unwrap()
    });
    assert!(extra.contains("not listed in the manifest"), "{extra}");
    let missing = tampered("t5-missing", &|src| {
        std::fs::remove_file(src.join("problems/bulk-email/hints.md")).unwrap()
    });
    assert!(
        missing.contains("listed in the manifest but missing"),
        "{missing}"
    );
    let outside = tampered("t5-outside", &|src| {
        std::fs::write(src.join("notes.md"), "x").unwrap()
    });
    assert!(outside.contains("packs hold problems/ only"), "{outside}");
}

#[test]
fn t6_bad_metadata_and_spoofed_ids_are_refused() {
    let p = problem_copy("t6");
    let err = |name: &str, id: &str, built_in: &[&str]| {
        pack_and_open(name, std::slice::from_ref(&p), id, built_in)
            .unwrap_err()
            .join("\n")
    };
    // A built-in id, or the core content's.
    assert!(err("t6-builtin", "chat", &["chat"]).contains("taken by built-in content"));
    assert!(err("t6-core", "core", &[]).contains("taken by built-in content"));
    // An id that could look like a problem id.
    assert!(err("t6-id", "two-sum", &[]).contains("3-20 lowercase"));
    // A future app version.
    let dir = temp("t6-minapp");
    let out = dir.join("x.kbpack");
    kbpack::write_pack(std::slice::from_ref(&p), &head("drills", "99.0.0"), &out).unwrap();
    let src = dir.join("src");
    let files = archive::extract(&out, &src).unwrap();
    let e = kbpack::check_extracted(&src, &files, &BTreeSet::new(), "0.1.0", &registry())
        .unwrap_err()
        .join("\n");
    assert!(e.contains("needs KataBox Arch 99.0.0"), "{e}");
    // Control characters in the name, a licence that isn't an SPDX expression.
    let dir = temp("t6-text");
    let out = dir.join("x.kbpack");
    let mut h = head("drills", "0.1.0");
    h.name = "Bell\u{7}".into();
    h.licence = "<b>free</b>".into();
    kbpack::write_pack(std::slice::from_ref(&p), &h, &out).unwrap();
    let src = dir.join("src");
    let files = archive::extract(&out, &src).unwrap();
    let e = kbpack::check_extracted(&src, &files, &BTreeSet::new(), "0.1.0", &registry())
        .unwrap_err()
        .join("\n");
    assert!(
        e.contains("no control characters") && e.contains("SPDX"),
        "{e}"
    );
}

#[test]
fn t7_markup_in_pack_text_is_refused() {
    for (i, evil) in [
        "<img src=x onerror=alert(1)>",
        "[click](javascript:alert(1))",
        "![x](data:image/png;base64,AAAA)",
        "<script>fetch('http://example.invalid')</script>",
        "<iframe src=\"http://example.invalid\"></iframe>",
    ]
    .iter()
    .enumerate()
    {
        let p = problem_copy(&format!("t7-{i}"));
        let s = std::fs::read_to_string(p.join("statement.md")).unwrap();
        std::fs::write(p.join("statement.md"), format!("{s}\n{evil}\n")).unwrap();
        let err = pack_and_open(&format!("t7-{i}"), &[p], "drills", &[])
            .unwrap_err()
            .join("\n");
        assert!(err.contains("not allowed in pack text"), "{evil}: {err}");
    }
}

#[test]
fn t8_invalid_content_and_designs_are_refused() {
    // Structure: the same rules as built-in content (here, a broken design).
    let p = problem_copy("t8-struct");
    std::fs::write(p.join("reference.kbarch"), "{ not a design").unwrap();
    let err = pack_and_open("t8-struct", &[p], "drills", &[])
        .unwrap_err()
        .join("\n");
    assert!(err.contains("reference.kbarch"), "{err}");
    // An unknown field in a design: the strict parser refuses it.
    let p = problem_copy("t8-strict");
    let d = std::fs::read_to_string(p.join("naive.kbarch")).unwrap();
    std::fs::write(
        p.join("naive.kbarch"),
        d.replacen('{', "{\"run\": \"cmd\",", 1),
    )
    .unwrap();
    let err = pack_and_open("t8-strict", &[p], "drills", &[])
        .unwrap_err()
        .join("\n");
    assert!(err.contains("naive.kbarch"), "{err}");
    // The engine: a "naive" design that passes everything is refused (it must fail a
    // hidden test), as is a reference that fails.
    let p = problem_copy("t8-judge");
    std::fs::copy(p.join("reference.kbarch"), p.join("naive.kbarch")).unwrap();
    let pack = pack_and_open("t8-judge", &[p], "drills", &[]).unwrap_or_else(|e| panic!("{e:#?}"));
    let issues = kbpack::judge_pack(&pack, &registry()).join("\n");
    assert!(
        issues.contains("must fail at least one hidden test"),
        "{issues}"
    );
}

#[test]
fn t9_problems_that_would_run_too_long_are_refused() {
    let p = problem_copy("t9");
    let toml = std::fs::read_to_string(p.join("problem.toml")).unwrap();
    let evil = toml
        .replacen("duration_s = 120", "duration_s = 3600", 1)
        .replacen("peak_rps = 500", "peak_rps = 50000000", 1);
    assert_ne!(evil, toml, "the fixture really changes the problem");
    std::fs::write(p.join("problem.toml"), evil).unwrap();
    let err = pack_and_open("t9", &[p], "drills", &[])
        .unwrap_err()
        .join("\n");
    assert!(err.contains("tests last at most 120 s"), "{err}");
    assert!(err.contains("peak_rps is at most"), "{err}");
}

#[test]
fn the_app_imports_a_real_pack_and_judges_its_problem() {
    let p = problem_copy("app");
    let dir = temp("app-pack");
    let out = dir.join("drills.kbpack");
    kbpack::write_pack(std::slice::from_ref(&p), &head("drills", "0.1.0"), &out).unwrap();
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(async {
        let mut app = ka_app::App::open(ka_app::AppPaths {
            user_db: dir.join("user.db"),
            dev_data: false,
            content_db: None,
            packs_dir: Some(dir.join("packs")),
        })
        .await
        .unwrap();
        app.set_pack_opener(std::sync::Arc::new(kbpack::Opener));
        let preview = app
            .preview_pack(&out)
            .await
            .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(preview.problems[0].id, "drills--bulk-email");
        assert_eq!(preview.sha256.len(), 64);
        app.apply_pack(preview.token).await.unwrap();
        // The stored problem judges like a built-in one: the pack's reference is Accepted.
        let (def, ref_cost) = app.problem_for_judging("drills--bulk-email").await.unwrap();
        assert_eq!(def.id, "drills--bulk-email");
        let reference = ka_app::kbarch::parse(
            &std::fs::read(p.join("reference.kbarch")).unwrap(),
            &registry(),
        )
        .unwrap();
        let j = ka_judge::judge(&def, &reference, &registry(), ref_cost);
        assert_eq!(j.verdict, ka_judge::Verdict::Accepted);
        app.close().await;
    });
}
