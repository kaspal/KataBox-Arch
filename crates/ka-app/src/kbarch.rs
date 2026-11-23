//! `.kbarch` files (ADR 0003): pretty-printed JSON of a [`Design`], strict on read.
//!
//! - **Read:** the size is checked before reading; then `schema_version` decides: newer is
//!   refused, older is migrated (v1 → v2 → v3, one step per version, each tested with a
//!   file of the old version); then the strict parse (unknown fields refused) and
//!   [`ka_core::validate`].
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
    let value = migrate(value, version, registry)?;
    let design: Design = serde_json::from_value(value).map_err(invalid_file)?;
    check(&design, registry)?;
    Ok(design)
}

/// Bring an older file up to [`SCHEMA_VERSION`]. Each format change adds a step here (and
/// a test with a file of the old version).
fn migrate(mut value: Value, version: u64, registry: &Registry) -> Result<Value> {
    let mut at = version;
    if at == 1 {
        value = v1_to_v2(value);
        at = 2;
    }
    if at == 2 {
        value = v2_to_v3(value, registry);
        at = 3;
    }
    if at == 3 {
        value = v3_to_v4(value);
        at = 4;
    }
    match at {
        4 => Ok(value),
        v => Err(invalid_file(format!("unknown schema_version {v}"))),
    }
}

/// v3 → v4 (M8): zones may carry `region_tag` and `mesh`, both optional, so a v3 file is a
/// valid v4 file once its version says so.
fn v3_to_v4(mut value: Value) -> Value {
    if let Some(obj) = value.as_object_mut() {
        obj.insert("schema_version".into(), Value::from(4));
    }
    value
}

/// The M0 placeholder defaults that differ from today's calibrated ones (commit 6f0267c,
/// before M1.2): (kind, flavour or "" for any, prop, value). Before v3 every card stored
/// every default, so a card holding exactly one of these got it from the app, not the user.
/// Every other M0 default equals today's and is dropped by the "equals the default" rule.
pub const PLACEHOLDERS_V1: &[(&str, &str, &str, f64)] = &[
    ("cache", "", "service_ms", 0.5),
    ("app", "django", "concurrency", 64.0),
    ("sql", "mysql", "concurrency", 100.0),
    ("worker", "sidekiq", "concurrency", 8.0),
    ("serverless", "cloud_run", "concurrency_limit", 1000.0),
];

/// v2 → v3 (ADR 0003): cards keep only the props the user set.
/// - LB `algorithm = "ip_hash"` becomes `"key_hash"` (the same behaviour, renamed);
/// - a prop equal to its flavour's current default is dropped (no change in behaviour);
/// - a prop equal to an M0 placeholder default is dropped, so the card picks up the
///   calibrated value;
/// - autoscaling wrappers' policies are thinned the same way.
///
/// Unknown kinds, flavours or props are left for the strict parse to refuse.
fn v2_to_v3(mut value: Value, registry: &Registry) -> Value {
    let Some(obj) = value.as_object_mut() else {
        return value;
    };
    obj.insert("schema_version".into(), Value::from(3));
    let parse = |v: Option<&Value>| v.and_then(|v| v.as_str()).map(str::to_owned);
    let thin = |kind: &str, flavour: &str, props: &mut serde_json::Map<String, Value>| {
        let Ok(k) = serde_json::from_value::<ka_core::ComponentKind>(Value::from(kind)) else {
            return;
        };
        let flav = serde_json::from_value::<ka_core::Flavour>(Value::from(flavour)).ok();
        let spec = registry.kind(k);
        if kind == "lb" && props.get("algorithm").and_then(Value::as_str) == Some("ip_hash") {
            props.insert("algorithm".into(), Value::from("key_hash"));
        }
        props.retain(|key, v| {
            let v: &Value = v;
            let current = flav
                .and_then(|f| spec.flavour(f))
                .and_then(|f| f.overrides.iter().find(|o| &o.key == key))
                .map(|o| o.value.clone())
                .or_else(|| spec.prop(key).map(|p| p.default.clone()));
            let same = |d: &ka_core::PropValue| match (d, v) {
                (ka_core::PropValue::Number(a), Value::Number(b)) => b.as_f64() == Some(*a),
                (ka_core::PropValue::Bool(a), Value::Bool(b)) => a == b,
                (ka_core::PropValue::Text(a), Value::String(b)) => a == b,
                _ => false,
            };
            if current.as_ref().is_some_and(same) {
                return false;
            }
            let placeholder = PLACEHOLDERS_V1.iter().any(|(pk, pf, pp, pv)| {
                *pk == kind
                    && (pf.is_empty() || *pf == flavour)
                    && pp == key
                    && v.as_f64() == Some(*pv)
            });
            !placeholder
        });
    };
    if let Some(nodes) = obj.get_mut("nodes").and_then(Value::as_array_mut) {
        for n in nodes {
            let (Some(kind), Some(flavour)) = (parse(n.get("kind")), parse(n.get("flavour")))
            else {
                continue;
            };
            if let Some(props) = n.get_mut("props").and_then(Value::as_object_mut) {
                thin(&kind, &flavour, props);
            }
        }
    }
    if let Some(groups) = obj.get_mut("groups").and_then(Value::as_array_mut) {
        for g in groups {
            let Some(scaling) = g.get_mut("scaling").and_then(Value::as_object_mut) else {
                continue;
            };
            let Some(flavour) = parse(scaling.get("flavour")) else {
                continue;
            };
            if let Some(props) = scaling.get_mut("props").and_then(Value::as_object_mut) {
                thin("autoscale", &flavour, props);
            }
        }
    }
    value
}

/// Props removed from the registry in v2, by kind: SQL read replicas are cards now, and
/// a CDN's hit rate is computed from its TTL and the traffic's key skew.
const REMOVED_IN_V2: &[(&str, &str)] = &[("sql", "read_replicas"), ("cdn", "hit_rate")];

/// The card size the canvas draws, for sizing migrated autoscaling wrappers.
const CARD: (f64, f64) = (208.0, 64.0);
const WRAP_PAD: f64 = 32.0;

/// v1 → v2 (ADR 0003, ADR 0004 F2):
/// - each Autoscaling card becomes an Autoscale wrapper around the compute cards it was
///   wired to (or a default frame at its own position), keeping its flavour and props;
///   its edges are dropped, and the wrapped cards join the wrapper;
/// - the props removed in v2 are dropped.
///
/// Works on the raw JSON, so anything unexpected is left for the strict parse to refuse.
fn v1_to_v2(mut value: Value) -> Value {
    let num = |v: &Value, k: &str| v.get(k).and_then(Value::as_f64).unwrap_or(0.0);
    let Some(obj) = value.as_object_mut() else {
        return value;
    };
    obj.insert("schema_version".into(), Value::from(2));

    let nodes = obj
        .get("nodes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let edges = obj
        .get("edges")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let is_autoscale = |n: &Value| n.get("kind").and_then(Value::as_str) == Some("autoscale");
    let scalers: Vec<&Value> = nodes.iter().filter(|n| is_autoscale(n)).collect();
    let id_of = |v: &Value| {
        v.get("id")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned()
    };
    let scaler_ids: Vec<String> = scalers.iter().map(|n| id_of(n)).collect();

    let mut new_groups = Vec::new();
    let mut joins: Vec<(String, String)> = Vec::new(); // (card id, wrapper id)
    for s in &scalers {
        let sid = id_of(s);
        let targets: Vec<&Value> = edges
            .iter()
            .filter(|e| e.get("from").and_then(Value::as_str) == Some(sid.as_str()))
            .filter_map(|e| e.get("to").and_then(Value::as_str))
            .filter_map(|to| nodes.iter().find(|n| id_of(n) == to && !is_autoscale(n)))
            .collect();
        let pos = s.get("position").cloned().unwrap_or(Value::Null);
        let (position, size) = if targets.is_empty() {
            (pos.clone(), serde_json::json!({ "w": 360, "h": 240 }))
        } else {
            let xs = targets.iter().map(|t| num(&t["position"], "x"));
            let ys = targets.iter().map(|t| num(&t["position"], "y"));
            let (x0, y0) = (
                xs.clone().fold(f64::INFINITY, f64::min),
                ys.clone().fold(f64::INFINITY, f64::min),
            );
            let (x1, y1) = (
                xs.fold(f64::NEG_INFINITY, f64::max) + CARD.0,
                ys.fold(f64::NEG_INFINITY, f64::max) + CARD.1,
            );
            (
                serde_json::json!({ "x": x0 - WRAP_PAD, "y": y0 - WRAP_PAD }),
                serde_json::json!({ "w": x1 - x0 + 2.0 * WRAP_PAD, "h": y1 - y0 + 2.0 * WRAP_PAD }),
            )
        };
        let parent = targets
            .first()
            .and_then(|t| t.get("group_id").cloned())
            .or_else(|| s.get("group_id").cloned())
            .unwrap_or(Value::Null);
        for t in &targets {
            joins.push((id_of(t), sid.clone()));
        }
        new_groups.push(serde_json::json!({
            "id": sid,
            "kind": "autoscale",
            "label": s.get("label").cloned().unwrap_or(Value::from("")),
            "position": position,
            "size": size,
            "parent_id": parent,
            "scaling": {
                "flavour": s.get("flavour").cloned().unwrap_or(Value::Null),
                "props": s.get("props").cloned().unwrap_or(serde_json::json!({})),
            },
        }));
    }

    let kept_nodes: Vec<Value> = nodes
        .into_iter()
        .filter(|n| !is_autoscale(n))
        .map(|mut n| {
            let id = id_of(&n);
            let kind = n
                .get("kind")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned();
            if let Some(props) = n.get_mut("props").and_then(Value::as_object_mut) {
                for (k, prop) in REMOVED_IN_V2 {
                    if *k == kind {
                        props.remove(*prop);
                    }
                }
            }
            if let Some((_, wrapper)) = joins.iter().find(|(card, _)| *card == id) {
                n["group_id"] = Value::from(wrapper.clone());
            }
            n
        })
        .collect();
    let kept_edges: Vec<Value> = edges
        .into_iter()
        .filter(|e| {
            let touches = |k: &str| {
                e.get(k)
                    .and_then(Value::as_str)
                    .is_some_and(|id| scaler_ids.iter().any(|s| s == id))
            };
            !touches("from") && !touches("to")
        })
        .collect();
    obj.insert("nodes".into(), Value::from(kept_nodes));
    obj.insert("edges".into(), Value::from(kept_edges));
    if let Some(groups) = obj.get_mut("groups").and_then(Value::as_array_mut) {
        groups.extend(new_groups);
    }
    value
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
            scaling: None,
            region_tag: None,
            mesh: None,
        });
        d.groups.push(Group {
            id: "az-1".into(),
            kind: GroupKind::Az,
            label: "AZ a".into(),
            position: Point { x: 300.0, y: 100.0 },
            size: Size { w: 400.0, h: 300.0 },
            parent_id: Some("region-1".into()),
            scaling: None,
            region_tag: None,
            mesh: None,
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

    /// A v1 file as M0 wrote it: an Autoscaling card wired to two app servers, an SQL card
    /// with read replicas and a CDN with a hit rate.
    fn v1_file() -> Value {
        serde_json::json!({
            "schema_version": 1, "id": "d1", "name": "Old", "problem_id": null,
            "nodes": [
                { "id": "app-1", "kind": "app", "flavour": "go", "label": "", "position": { "x": 100, "y": 100 },
                  "props": {}, "group_id": "az-1" },
                { "id": "app-2", "kind": "app", "flavour": "go", "label": "", "position": { "x": 400, "y": 100 },
                  "props": {}, "group_id": "az-1" },
                { "id": "hpa", "kind": "autoscale", "flavour": "kubernetes_hpa", "label": "web tier",
                  "position": { "x": 250, "y": 300 }, "props": { "max_instances": 8 }, "group_id": null },
                { "id": "db", "kind": "sql", "flavour": "postgresql", "label": "", "position": { "x": 250, "y": 500 },
                  "props": { "read_replicas": 2, "shards": 1 }, "group_id": null },
                { "id": "cdn", "kind": "cdn", "flavour": "fastly", "label": "", "position": { "x": 0, "y": 0 },
                  "props": { "hit_rate": 90, "ttl_s": 60 }, "group_id": null }
            ],
            "edges": [
                { "id": "e1", "from": "hpa", "to": "app-1", "protocol": "http" },
                { "id": "e2", "from": "hpa", "to": "app-2", "protocol": "http" },
                { "id": "e3", "from": "app-1", "to": "db", "protocol": "query" }
            ],
            "groups": [
                { "id": "az-1", "kind": "az", "label": "AZ a", "position": { "x": 0, "y": 0 },
                  "size": { "w": 800, "h": 400 }, "parent_id": null }
            ],
            "notes": [],
            "viewport": { "x": 0, "y": 0, "zoom": 1 }
        })
    }

    #[test]
    fn v1_autoscaling_cards_become_wrappers_around_what_they_scaled() {
        let d = parse(v1_file().to_string().as_bytes(), &registry()).unwrap();
        assert_eq!(d.schema_version, SCHEMA_VERSION);
        assert!(d.nodes.iter().all(|n| n.kind != ComponentKind::Autoscale));
        assert_eq!(
            d.edges.iter().map(|e| e.id.as_str()).collect::<Vec<_>>(),
            ["e3"]
        );
        let w = d
            .groups
            .iter()
            .find(|g| g.id == "hpa")
            .expect("the wrapper");
        assert_eq!(w.kind, GroupKind::Autoscale);
        assert_eq!(w.label, "web tier");
        assert_eq!(w.parent_id.as_deref(), Some("az-1"));
        let s = w.scaling.as_ref().unwrap();
        assert_eq!(s.flavour, Flavour::KubernetesHpa);
        assert_eq!(s.props.get("max_instances"), Some(&PropValue::Number(8.0)));
        // The frame wraps both apps with padding; both joined it.
        assert_eq!(
            (w.position.x, w.position.y),
            (100.0 - WRAP_PAD, 100.0 - WRAP_PAD)
        );
        assert_eq!(
            (w.size.w, w.size.h),
            (300.0 + CARD.0 + 2.0 * WRAP_PAD, CARD.1 + 2.0 * WRAP_PAD)
        );
        assert!(
            d.nodes
                .iter()
                .filter(|n| n.kind == ComponentKind::App)
                .all(|n| n.group_id.as_deref() == Some("hpa"))
        );
        // Re-saving writes v2, which reads back identically.
        let bytes = to_bytes(&d, &registry()).unwrap();
        assert_eq!(parse(&bytes, &registry()).unwrap(), d);
    }

    #[test]
    fn an_unwired_v1_autoscaler_becomes_a_frame_at_its_position() {
        let mut v = v1_file();
        v["edges"] = serde_json::json!([]);
        let d = parse(v.to_string().as_bytes(), &registry()).unwrap();
        let w = d.groups.iter().find(|g| g.id == "hpa").unwrap();
        assert_eq!((w.position.x, w.position.y), (250.0, 300.0));
        assert_eq!(w.parent_id, None);
    }

    #[test]
    fn v2_drops_sql_read_replicas() {
        let d = parse(v1_file().to_string().as_bytes(), &registry()).unwrap();
        let db = d.nodes.iter().find(|n| n.id == "db").unwrap();
        assert!(!db.props.contains_key("read_replicas"));
        // Then v3 drops `shards = 1`: it is the default.
        assert!(db.props.is_empty(), "{:?}", db.props);
    }

    #[test]
    fn v2_drops_cdn_hit_rate() {
        let d = parse(v1_file().to_string().as_bytes(), &registry()).unwrap();
        let cdn = d.nodes.iter().find(|n| n.id == "cdn").unwrap();
        assert!(!cdn.props.contains_key("hit_rate"));
        assert_eq!(cdn.props.get("ttl_s"), Some(&PropValue::Number(60.0)));
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
