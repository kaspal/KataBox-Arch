//! `.kbarch` files (ADR 0003): pretty-printed JSON of a [`Design`], strict on read.
//!
//! - **Read:** the size is checked before reading; then `schema_version` decides: newer is
//!   refused, older is migrated (none yet: v1 is the first); then the strict parse
//!   (unknown fields refused) and [`ka_core::validate`].
//! - **Write:** validate first, then write a temp file next to the target and rename it
//!   over the target, so a crash never leaves a half-written design.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use ka_core::{Design, Registry, SCHEMA_VERSION, validate};
use serde_json::Value;

use crate::error::{AppError, AppErrorCode};

/// The largest `.kbarch` file the app opens.
pub const KBARCH_MAX_BYTES: u64 = 8 * 1024 * 1024;

/// The file extension, without the dot.
pub const EXTENSION: &str = "kbarch";

type Result<T> = std::result::Result<T, AppError>;

fn invalid_file(msg: impl std::fmt::Display) -> AppError {
    AppError::new(
        AppErrorCode::InvalidFile,
        format!("not a valid .kbarch file: {msg}"),
    )
}

/// Refuse a design with issues, naming the first few.
pub fn check(design: &Design, registry: &Registry) -> Result<()> {
    let issues = validate(design, registry);
    if issues.is_empty() {
        return Ok(());
    }
    let first: Vec<String> = issues
        .iter()
        .take(5)
        .map(|i| {
            let code = serde_json::to_value(i.code)
                .ok()
                .and_then(|v| v.as_str().map(str::to_owned));
            format!("{} ({})", code.unwrap_or_default(), i.ids.join(", "))
        })
        .collect();
    Err(AppError::new(
        AppErrorCode::InvalidDesign,
        format!("{} issue(s): {}", issues.len(), first.join("; ")),
    ))
}

/// Parse and validate a file's bytes.
pub fn parse(bytes: &[u8], registry: &Registry) -> Result<Design> {
    let value: Value = serde_json::from_slice(bytes).map_err(invalid_file)?;
    let version = value
        .get("schema_version")
        .and_then(Value::as_u64)
        .ok_or_else(|| invalid_file("missing schema_version"))?;
    if version > u64::from(SCHEMA_VERSION) {
        return Err(AppError::new(
            AppErrorCode::NewerFileSchema,
            format!("the file is format v{version}; this app reads up to v{SCHEMA_VERSION}"),
        ));
    }
    let value = migrate(value, version)?;
    let design: Design = serde_json::from_value(value).map_err(invalid_file)?;
    check(&design, registry)?;
    Ok(design)
}

/// Bring an older file up to [`SCHEMA_VERSION`]. Each future format change adds a step
/// here (and a test with a file of the old version).
fn migrate(value: Value, version: u64) -> Result<Value> {
    match version {
        1 => Ok(value),
        v => Err(invalid_file(format!("unknown schema_version {v}"))),
    }
}

/// The bytes written for a design: validated, pretty JSON, a final newline. The same
/// design always gives the same bytes.
pub fn to_bytes(design: &Design, registry: &Registry) -> Result<Vec<u8>> {
    check(design, registry)?;
    if design.schema_version != SCHEMA_VERSION {
        return Err(AppError::new(
            AppErrorCode::InvalidInput,
            format!("schema_version must be {SCHEMA_VERSION}"),
        ));
    }
    let mut out = serde_json::to_vec_pretty(design)
        .map_err(|e| AppError::new(AppErrorCode::Internal, e.to_string()))?;
    out.push(b'\n');
    Ok(out)
}

/// Read a design from disk (size-capped, strict).
pub fn read(path: &Path, registry: &Registry) -> Result<Design> {
    let file = std::fs::File::open(path)?;
    if file.metadata()?.len() > KBARCH_MAX_BYTES {
        return Err(too_large());
    }
    // `take` guards against a file that grows between the check and the read.
    let mut bytes = Vec::new();
    file.take(KBARCH_MAX_BYTES + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > KBARCH_MAX_BYTES {
        return Err(too_large());
    }
    parse(&bytes, registry)
}

fn too_large() -> AppError {
    AppError::new(
        AppErrorCode::FileTooLarge,
        format!(
            "the file is larger than {} MiB",
            KBARCH_MAX_BYTES / 1024 / 1024
        ),
    )
}

/// Write a design atomically: `<target>.tmp`, flushed, then renamed over the target.
pub fn write(path: &Path, design: &Design, registry: &Registry) -> Result<()> {
    let bytes = to_bytes(design, registry)?;
    let tmp = temp_path(path);
    let written = (|| -> std::io::Result<()> {
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(&bytes)?;
        f.sync_all()?;
        drop(f);
        std::fs::rename(&tmp, path)
    })();
    if let Err(e) = written {
        let _ = std::fs::remove_file(&tmp);
        return Err(e.into());
    }
    Ok(())
}

fn temp_path(path: &Path) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".tmp");
    path.with_file_name(name)
}

/// `path` with the `.kbarch` extension (added if the dialog didn't).
pub fn with_extension(path: PathBuf) -> PathBuf {
    let has = path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case(EXTENSION));
    if has {
        path
    } else {
        let mut s = path.into_os_string();
        s.push(".");
        s.push(EXTENSION);
        PathBuf::from(s)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::collections::BTreeMap;

    use ka_core::{
        ComponentKind, Edge, Flavour, Group, GroupKind, Node, Note, Point, PropValue, Protocol,
        Size, registry,
    };

    use super::*;
    use crate::testutil::temp_dir;

    fn node(id: &str, kind: ComponentKind, flavour: Flavour, x: f64, y: f64) -> Node {
        Node {
            id: id.into(),
            kind,
            flavour,
            label: String::new(),
            position: Point { x, y },
            props: BTreeMap::new(),
            group_id: None,
        }
    }

    /// A design with every kind of item: zones nested Region ⊃ AZ, a card in the AZ,
    /// a note, props of each type and fractional coordinates.
    pub(crate) fn sample() -> Design {
        let mut d = Design::empty("design-1", "URL Shortener ñ");
        d.groups.push(Group {
            id: "region-1".into(),
            kind: GroupKind::Region,
            label: "us-east-1".into(),
            position: Point { x: 280.0, y: 60.0 },
            size: Size { w: 760.0, h: 520.0 },
            parent_id: None,
        });
        d.groups.push(Group {
            id: "az-1".into(),
            kind: GroupKind::Az,
            label: "AZ a".into(),
            position: Point { x: 300.0, y: 100.0 },
            size: Size { w: 400.0, h: 300.0 },
            parent_id: Some("region-1".into()),
        });
        d.nodes.push(node(
            "client-1",
            ComponentKind::Client,
            Flavour::Web,
            40.0,
            300.5,
        ));
        let mut app = node("app-1", ComponentKind::App, Flavour::Go, 340.25, 160.0);
        app.label = "App ×3".into();
        app.group_id = Some("az-1".into());
        app.props.insert("instances".into(), PropValue::Number(3.0));
        app.props
            .insert("service_ms".into(), PropValue::Number(4.5));
        d.nodes.push(app);
        let mut cache = node("cache-1", ComponentKind::Cache, Flavour::Redis, 720.0, 80.0);
        cache.group_id = Some("region-1".into());
        cache
            .props
            .insert("eviction".into(), PropValue::Text("lfu".into()));
        cache
            .props
            .insert("persistence".into(), PropValue::Bool(true));
        cache
            .props
            .insert("memory_gb".into(), PropValue::Number(2.0));
        d.nodes.push(cache);
        d.nodes.push(node(
            "db-1",
            ComponentKind::Sql,
            Flavour::Postgresql,
            720.0,
            420.0,
        ));
        d.nodes.push(node(
            "db-2",
            ComponentKind::Sql,
            Flavour::Postgresql,
            520.0,
            520.0,
        ));
        let e = |id: &str, f: &str, t: &str, p| Edge {
            id: id.into(),
            from: f.into(),
            to: t.into(),
            protocol: p,
        };
        d.edges.push(e("e1", "client-1", "app-1", Protocol::Http));
        d.edges.push(e("e2", "app-1", "cache-1", Protocol::Query));
        d.edges.push(e("e3", "app-1", "db-1", Protocol::Query));
        d.edges.push(e("e4", "db-1", "db-2", Protocol::Replicate));
        d.notes.push(Note {
            id: "note-1".into(),
            text: "Reads are 95% of traffic.\nCache first.".into(),
            position: Point { x: 40.0, y: 480.0 },
            size: Size { w: 220.0, h: 90.0 },
        });
        d.viewport = ka_core::Viewport {
            x: -12.5,
            y: 30.0,
            zoom: 0.85,
        };
        d
    }

    #[test]
    fn round_trips_identically_and_writes_stable_bytes() {
        let r = registry();
        let dir = temp_dir("roundtrip");
        let path = dir.join("shortener.kbarch");
        let d = sample();
        write(&path, &d, &r).unwrap();
        let back = read(&path, &r).unwrap();
        assert_eq!(back, d);
        // Nested zones keep absolute positions and memberships.
        assert_eq!(back.nodes[1].group_id.as_deref(), Some("az-1"));
        assert_eq!(back.groups[1].parent_id.as_deref(), Some("region-1"));
        // Save → open → save gives the same bytes; no temp file is left behind.
        let first = std::fs::read(&path).unwrap();
        write(&path, &back, &r).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), first);
        assert!(!temp_path(&path).exists());
        let text = String::from_utf8(first).unwrap();
        assert!(
            text.contains("\"instances\": 3"),
            "whole numbers have no .0:\n{text}"
        );
        assert!(text.ends_with("}\n"));
    }

    #[test]
    fn a_float_prop_saved_as_a_whole_number_reads_back() {
        let r = registry();
        let mut d = sample();
        d.nodes[1]
            .props
            .insert("service_ms".into(), PropValue::Number(2.0));
        let bytes = to_bytes(&d, &r).unwrap();
        assert_eq!(parse(&bytes, &r).unwrap(), d);
    }

    fn refusal(json: &str) -> AppErrorCode {
        parse(json.as_bytes(), &registry()).unwrap_err().code
    }

    fn sample_json() -> Value {
        serde_json::to_value(sample()).unwrap()
    }

    #[test]
    fn strict_import_refusals() {
        // Unknown field at the top level and deep inside.
        let mut v = sample_json();
        v["extra"] = Value::from(1);
        assert_eq!(refusal(&v.to_string()), AppErrorCode::InvalidFile);
        let mut v = sample_json();
        v["nodes"][0]["colour"] = Value::from("red");
        assert_eq!(refusal(&v.to_string()), AppErrorCode::InvalidFile);
        // Missing field, wrong type, unknown kind or flavour name.
        let mut v = sample_json();
        v.as_object_mut().unwrap().remove("edges");
        assert_eq!(refusal(&v.to_string()), AppErrorCode::InvalidFile);
        let mut v = sample_json();
        v["nodes"][0]["kind"] = Value::from("mainframe");
        assert_eq!(refusal(&v.to_string()), AppErrorCode::InvalidFile);
        // Not JSON, no version, a newer version.
        assert_eq!(refusal("not json"), AppErrorCode::InvalidFile);
        assert_eq!(refusal("{}"), AppErrorCode::InvalidFile);
        let mut v = sample_json();
        v["schema_version"] = Value::from(SCHEMA_VERSION + 1);
        assert_eq!(refusal(&v.to_string()), AppErrorCode::NewerFileSchema);
        // Parses, but breaks a design rule.
        let mut v = sample_json();
        v["edges"][0]["to"] = Value::from("db-1"); // client → SQL: not allowed
        v["edges"][0]["protocol"] = Value::from("query");
        assert_eq!(refusal(&v.to_string()), AppErrorCode::InvalidDesign);
        let mut v = sample_json();
        v["notes"][0]["id"] = Value::from("app-1");
        assert_eq!(refusal(&v.to_string()), AppErrorCode::InvalidDesign);
        let mut v = sample_json();
        v["nodes"][1]["props"]["instances"] = Value::from(2.5);
        assert_eq!(refusal(&v.to_string()), AppErrorCode::InvalidDesign);
    }

    #[test]
    fn oversize_files_are_refused_before_reading() {
        let dir = temp_dir("oversize");
        let path = dir.join("big.kbarch");
        let f = std::fs::File::create(&path).unwrap();
        f.set_len(KBARCH_MAX_BYTES + 1).unwrap();
        drop(f);
        assert_eq!(
            read(&path, &registry()).unwrap_err().code,
            AppErrorCode::FileTooLarge
        );
        assert_eq!(
            read(&dir.join("missing.kbarch"), &registry())
                .unwrap_err()
                .code,
            AppErrorCode::NotFound
        );
    }

    #[test]
    fn an_invalid_design_is_never_written() {
        let dir = temp_dir("invalid-write");
        let path = dir.join("d.kbarch");
        let mut d = sample();
        d.edges[0].from = "nowhere".into();
        assert_eq!(
            write(&path, &d, &registry()).unwrap_err().code,
            AppErrorCode::InvalidDesign
        );
        assert!(!path.exists());
        assert!(!temp_path(&path).exists());
    }

    #[test]
    fn the_extension_is_added_once() {
        assert_eq!(with_extension("a/b".into()), PathBuf::from("a/b.kbarch"));
        assert_eq!(
            with_extension("a/b.KBARCH".into()),
            PathBuf::from("a/b.KBARCH")
        );
        assert_eq!(
            with_extension("a/b.json".into()),
            PathBuf::from("a/b.json.kbarch")
        );
    }
}
