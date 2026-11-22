//! Mermaid export (M6.6): a design as a `flowchart LR` diagram, for docs and wikis.
//!
//! - Cards become nodes labelled "Label (Kind)" (just the kind when the label is empty);
//!   the kind names come from the caller, in the reader's language.
//! - Regions, AZs and autoscaling groups become nested `subgraph`s.
//! - Edges keep their direction and carry their protocol's name; async messages are
//!   dashed and replication dotted, as on the canvas.
//! - Ids are rewritten to `[A-Za-z0-9_]` (unique), and labels are escaped, so no text in a
//!   design can break out of the diagram.
//!
//! Deterministic: the same design and names always give the same text.

use std::collections::{BTreeMap, BTreeSet};

use crate::design::{Design, GroupKind, Protocol};

/// The names to print: `kind.<id>` → "App Server", `protocol.<id>` → "HTTP",
/// `group.region` / `group.az` / `group.autoscale` → "Region"… Missing ones fall back to
/// the id.
pub type Names = BTreeMap<String, String>;

fn protocol_id(p: Protocol) -> &'static str {
    match p {
        Protocol::Http => "http",
        Protocol::Grpc => "grpc",
        Protocol::Ws => "ws",
        Protocol::Query => "query",
        Protocol::Publish => "publish",
        Protocol::Consume => "consume",
        Protocol::Replicate => "replicate",
    }
}

fn group_id(k: GroupKind) -> &'static str {
    match k {
        GroupKind::Region => "region",
        GroupKind::Az => "az",
        GroupKind::Autoscale => "autoscale",
    }
}

/// Text safe inside a Mermaid `"…"` label: quotes, angle brackets, `#`, backticks and
/// line breaks become entity codes or spaces.
fn escape(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        match c {
            '"' => out.push_str("#quot;"),
            '<' => out.push_str("#lt;"),
            '>' => out.push_str("#gt;"),
            '#' => out.push_str("#35;"),
            '`' => out.push_str("#96;"),
            '|' => out.push_str("#124;"),
            '\n' | '\r' | '\t' => out.push(' '),
            c if c.is_control() => {}
            c => out.push(c),
        }
    }
    out
}

/// Unique Mermaid ids for the design's ids, with a prefix (`n_`, `g_`).
fn ids<'a>(prefix: &str, raw: impl Iterator<Item = &'a str>) -> BTreeMap<&'a str, String> {
    let mut used = BTreeSet::new();
    let mut out = BTreeMap::new();
    for id in raw {
        let clean: String = id
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
            .collect();
        let mut candidate = format!("{prefix}{clean}");
        let mut n = 2;
        while !used.insert(candidate.clone()) {
            candidate = format!("{prefix}{clean}_{n}");
            n += 1;
        }
        out.insert(id, candidate);
    }
    out
}

pub fn to_mermaid(design: &Design, names: &Names) -> String {
    let name = |key: String, fallback: &str| {
        names
            .get(&key)
            .cloned()
            .unwrap_or_else(|| fallback.to_owned())
    };
    let node_ids = ids("n_", design.nodes.iter().map(|n| n.id.as_str()));
    let group_ids = ids("g_", design.groups.iter().map(|g| g.id.as_str()));
    let mut out = String::from("flowchart LR\n");

    // Cards and groups, nested: a group holds its cards, then its child groups.
    fn emit(
        out: &mut String,
        design: &Design,
        parent: Option<&str>,
        depth: usize,
        node_ids: &BTreeMap<&str, String>,
        group_ids: &BTreeMap<&str, String>,
        name: &dyn Fn(String, &str) -> String,
    ) {
        let pad = "  ".repeat(depth);
        for n in design
            .nodes
            .iter()
            .filter(|n| n.group_id.as_deref() == parent)
        {
            let kind = name(format!("kind.{}", n.kind.id()), n.kind.id());
            let label = if n.label.trim().is_empty() {
                escape(&kind)
            } else {
                format!("{} ({})", escape(n.label.trim()), escape(&kind))
            };
            out.push_str(&format!("{pad}{}[\"{label}\"]\n", node_ids[n.id.as_str()]));
        }
        for g in design
            .groups
            .iter()
            .filter(|g| g.parent_id.as_deref() == parent)
        {
            let kind = name(format!("group.{}", group_id(g.kind)), group_id(g.kind));
            let label = if g.label.trim().is_empty() {
                escape(&kind)
            } else {
                format!("{} ({})", escape(g.label.trim()), escape(&kind))
            };
            out.push_str(&format!(
                "{pad}subgraph {}[\"{label}\"]\n",
                group_ids[g.id.as_str()]
            ));
            emit(
                out,
                design,
                Some(&g.id),
                depth + 1,
                node_ids,
                group_ids,
                name,
            );
            out.push_str(&format!("{pad}end\n"));
        }
    }
    // Cards whose group is missing are drawn at the top level rather than dropped.
    let known: BTreeSet<&str> = design.groups.iter().map(|g| g.id.as_str()).collect();
    let mut flat = design.clone();
    for n in &mut flat.nodes {
        if n.group_id.as_deref().is_some_and(|g| !known.contains(g)) {
            n.group_id = None;
        }
    }
    for g in &mut flat.groups {
        if g.parent_id.as_deref().is_some_and(|p| !known.contains(p)) {
            g.parent_id = None;
        }
    }
    emit(&mut out, &flat, None, 1, &node_ids, &group_ids, &name);

    for e in &design.edges {
        let (Some(a), Some(b)) = (node_ids.get(e.from.as_str()), node_ids.get(e.to.as_str()))
        else {
            continue;
        };
        let p = protocol_id(e.protocol);
        let label = escape(&name(format!("protocol.{p}"), p));
        let arrow = match e.protocol {
            Protocol::Publish | Protocol::Consume => "-.->",
            Protocol::Replicate => "-.-",
            _ => "-->",
        };
        out.push_str(&format!("  {a} {arrow}|{label}| {b}\n"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::design::{Edge, Group, Node, Point, Size};
    use crate::{ComponentKind as K, Flavour};

    fn node(id: &str, kind: K, flavour: Flavour, label: &str, group: Option<&str>) -> Node {
        Node {
            id: id.into(),
            kind,
            flavour,
            label: label.into(),
            position: Point { x: 0.0, y: 0.0 },
            props: Default::default(),
            group_id: group.map(Into::into),
        }
    }

    fn group(id: &str, kind: GroupKind, label: &str, parent: Option<&str>) -> Group {
        Group {
            id: id.into(),
            kind,
            label: label.into(),
            position: Point { x: 0.0, y: 0.0 },
            size: Size { w: 100.0, h: 100.0 },
            parent_id: parent.map(Into::into),
            scaling: None,
        }
    }

    fn edge(from: &str, to: &str, protocol: Protocol) -> Edge {
        Edge {
            id: format!("{from}-{to}"),
            from: from.into(),
            to: to.into(),
            protocol,
        }
    }

    #[test]
    fn a_design_becomes_a_flowchart_with_nested_zones() {
        let mut d = Design::empty("d", "D");
        d.groups = vec![
            group("us", GroupKind::Region, "us-east", None),
            group("a", GroupKind::Az, "us-east-1a", Some("us")),
        ];
        d.nodes = vec![
            node("client", K::Client, Flavour::Web, "", None),
            node("app", K::App, Flavour::Go, "API", Some("a")),
            node("db", K::Sql, Flavour::Postgresql, "Primary", Some("a")),
            node("db-2", K::Sql, Flavour::Postgresql, "Replica", Some("us")),
            node("q", K::Queue, Flavour::Sqs, "", None),
        ];
        d.edges = vec![
            edge("client", "app", Protocol::Http),
            edge("app", "db", Protocol::Query),
            edge("db", "db-2", Protocol::Replicate),
            edge("app", "q", Protocol::Publish),
        ];
        let names: Names = [
            ("kind.client", "Client"),
            ("kind.app", "App Server"),
            ("kind.sql", "SQL Database"),
            ("protocol.http", "HTTP"),
            ("group.region", "Region"),
            ("group.az", "AZ"),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .collect();
        let text = to_mermaid(&d, &names);
        assert_eq!(
            text,
            "flowchart LR
  n_client[\"Client\"]
  n_q[\"queue\"]
  subgraph g_us[\"us-east (Region)\"]
    n_db_2[\"Replica (SQL Database)\"]
    subgraph g_a[\"us-east-1a (AZ)\"]
      n_app[\"API (App Server)\"]
      n_db[\"Primary (SQL Database)\"]
    end
  end
  n_client -->|HTTP| n_app
  n_app -->|query| n_db
  n_db -.-|replicate| n_db_2
  n_app -.->|publish| n_q
"
        );
        assert_eq!(to_mermaid(&d, &names), text, "deterministic");
    }

    #[test]
    fn labels_and_ids_cannot_break_out() {
        let mut d = Design::empty("d", "D");
        d.nodes = vec![
            node(
                "a b",
                K::App,
                Flavour::Go,
                "x\"]\n  click a call alert(1)\n%%",
                None,
            ),
            node("a_b", K::App, Flavour::Go, "<b>#`|", None),
        ];
        d.edges = vec![edge("a b", "a_b", Protocol::Http)];
        let text = to_mermaid(&d, &Names::new());
        assert!(
            text.contains("n_a_b[\"x#quot;]   click a call alert(1) %% (app)\"]"),
            "{text}"
        );
        assert!(
            text.contains("n_a_b_2[\"#lt;b#gt;#35;#96;#124; (app)\"]"),
            "{text}"
        );
        assert!(text.contains("n_a_b -->|http| n_a_b_2"), "{text}");
        assert_eq!(
            text.lines().count(),
            4,
            "one line per card and edge: {text}"
        );
    }
}
