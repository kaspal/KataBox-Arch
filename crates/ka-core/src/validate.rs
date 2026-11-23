//! Design validation: everything a `.kbarch` file must satisfy before it is saved or
//! opened (ADR 0003). Returns every issue found, not just the first.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::design::{Design, GroupKind, Point, Size};
use crate::registry::{ComponentKind, Placement, Registry};
use crate::wiring::WiringReason;

/// Size caps for a design. Sent to the UI with the registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Limits {
    pub max_nodes: u32,
    pub max_edges: u32,
    pub max_groups: u32,
    pub max_notes: u32,
    /// Characters, for design names, labels and zone labels.
    pub max_label: u32,
    /// Characters, for a note's text.
    pub max_note: u32,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_nodes: 500,
            max_edges: 2000,
            max_groups: 100,
            max_notes: 200,
            max_label: 120,
            max_note: 4000,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum IssueCode {
    /// An id is empty, too long, or uses characters outside `[A-Za-z0-9_-]`.
    BadId,
    DuplicateId,
    /// An edge names a card that doesn't exist.
    UnknownNode,
    SelfLoop,
    DuplicateEdge,
    /// The wiring table doesn't allow this protocol between these kinds.
    WiringNotAllowed,
    /// The flavour doesn't belong to the card's kind.
    UnknownFlavour,
    UnknownProp,
    /// A prop has the wrong type (number, bool, choice).
    PropType,
    PropOutOfRange,
    /// A card or zone names a zone that doesn't exist.
    UnknownGroup,
    /// Zones nest in a loop.
    GroupCycle,
    TextTooLong,
    /// A coordinate, size or zoom is NaN, infinite, or not positive where it must be.
    BadNumber,
    TooManyItems,
    /// An Autoscale wrapper without a valid scaling policy, or a policy on another zone.
    BadScaling,
    /// A wrapper kind (Autoscaling) placed as a card.
    WrapperAsCard,
    /// A residency tag on a zone that isn't a Region, or a malformed tag; or a service mesh on
    /// a zone that isn't a Region or AZ, or with an unknown or out-of-range prop (v4).
    BadZoneSetting,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct DesignIssue {
    pub code: IssueCode,
    /// The ids involved (cards, edges, zones, notes), or the prop key.
    pub ids: Vec<String>,
}

fn issue(code: IssueCode, ids: &[&str]) -> DesignIssue {
    DesignIssue {
        code,
        ids: ids.iter().map(|s| (*s).to_owned()).collect(),
    }
}

fn good_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

fn finite_point(p: &Point) -> bool {
    p.x.is_finite() && p.y.is_finite()
}

fn good_size(s: &Size) -> bool {
    s.w.is_finite() && s.h.is_finite() && s.w > 0.0 && s.h > 0.0
}

fn chars(s: &str) -> u32 {
    u32::try_from(s.chars().count()).unwrap_or(u32::MAX)
}

fn count(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// An Autoscale wrapper needs a policy with an Autoscaling flavour and valid props;
/// other zones have none.
fn scaling_ok(g: &crate::design::Group, registry: &Registry) -> bool {
    match (g.kind, &g.scaling) {
        (GroupKind::Autoscale, Some(s)) => {
            let spec = registry.kind(ComponentKind::Autoscale);
            spec.flavour(s.flavour).is_some()
                && s.props
                    .iter()
                    .all(|(k, val)| spec.prop(k).is_some_and(|p| p.accepts(val)))
        }
        (GroupKind::Autoscale, None) => false,
        (_, scaling) => scaling.is_none(),
    }
}

/// A residency tag (`eu`, `us`, `apac`…): 2–8 lowercase letters, on a Region only. A mesh sits
/// on a Region or AZ, with props from `registry.mesh_props`.
fn zone_settings_ok(g: &crate::design::Group, registry: &Registry) -> bool {
    let tag_ok = g.region_tag.as_deref().is_none_or(|t| {
        g.kind == GroupKind::Region
            && (2..=8).contains(&t.len())
            && t.bytes().all(|b| b.is_ascii_lowercase())
    });
    let mesh_ok = g.mesh.as_ref().is_none_or(|m| {
        matches!(g.kind, GroupKind::Region | GroupKind::Az)
            && m.props.iter().all(|(k, val)| {
                registry
                    .mesh_props
                    .iter()
                    .find(|p| &p.key == k)
                    .is_some_and(|p| p.accepts(val))
            })
    });
    tag_ok && mesh_ok
}

/// Every problem with `design`, in a stable order. Empty means valid.
pub fn validate(design: &Design, registry: &Registry) -> Vec<DesignIssue> {
    let limits = &registry.limits;
    let mut out = Vec::new();

    // --- Sizes and text ---
    for (n, max, what) in [
        (design.nodes.len(), limits.max_nodes, "nodes"),
        (design.edges.len(), limits.max_edges, "edges"),
        (design.groups.len(), limits.max_groups, "groups"),
        (design.notes.len(), limits.max_notes, "notes"),
    ] {
        if count(n) > max {
            out.push(issue(IssueCode::TooManyItems, &[what]));
        }
    }
    if chars(&design.name) > limits.max_label {
        out.push(issue(IssueCode::TextTooLong, &[&design.id]));
    }
    let vp = &design.viewport;
    if !(vp.x.is_finite() && vp.y.is_finite() && vp.zoom.is_finite() && vp.zoom > 0.0) {
        out.push(issue(IssueCode::BadNumber, &["viewport"]));
    }

    // --- Ids: well formed, unique across every item ---
    let mut seen = HashSet::new();
    let ids = std::iter::once(design.id.as_str())
        .chain(design.nodes.iter().map(|n| n.id.as_str()))
        .chain(design.edges.iter().map(|e| e.id.as_str()))
        .chain(design.groups.iter().map(|g| g.id.as_str()))
        .chain(design.notes.iter().map(|n| n.id.as_str()));
    for id in ids {
        if !good_id(id) {
            out.push(issue(IssueCode::BadId, &[id]));
        } else if !seen.insert(id) {
            out.push(issue(IssueCode::DuplicateId, &[id]));
        }
    }

    // --- Zones ---
    let groups: HashMap<&str, &crate::design::Group> =
        design.groups.iter().map(|g| (g.id.as_str(), g)).collect();
    for g in &design.groups {
        if chars(&g.label) > limits.max_label {
            out.push(issue(IssueCode::TextTooLong, &[&g.id]));
        }
        if !finite_point(&g.position) || !good_size(&g.size) {
            out.push(issue(IssueCode::BadNumber, &[&g.id]));
        }
        if let Some(parent) = &g.parent_id
            && !groups.contains_key(parent.as_str())
        {
            out.push(issue(IssueCode::UnknownGroup, &[&g.id, parent]));
        }
        if !scaling_ok(g, registry) {
            out.push(issue(IssueCode::BadScaling, &[&g.id]));
        }
        if !zone_settings_ok(g, registry) {
            out.push(issue(IssueCode::BadZoneSetting, &[&g.id]));
        }
    }
    for g in &design.groups {
        // Walk up the parents; more steps than zones means a loop.
        let mut at = g.parent_id.as_deref();
        let mut steps = 0;
        while let Some(p) = at {
            steps += 1;
            if p == g.id || steps > design.groups.len() {
                out.push(issue(IssueCode::GroupCycle, &[&g.id]));
                break;
            }
            at = groups.get(p).and_then(|pg| pg.parent_id.as_deref());
        }
    }

    // --- Cards ---
    let nodes: HashMap<&str, &crate::design::Node> =
        design.nodes.iter().map(|n| (n.id.as_str(), n)).collect();
    for n in &design.nodes {
        let spec = registry.kind(n.kind);
        if spec.placement == Placement::Wrapper {
            out.push(issue(IssueCode::WrapperAsCard, &[&n.id]));
        }
        if spec.flavour(n.flavour).is_none() {
            out.push(issue(IssueCode::UnknownFlavour, &[&n.id]));
        }
        if chars(&n.label) > limits.max_label {
            out.push(issue(IssueCode::TextTooLong, &[&n.id]));
        }
        if !finite_point(&n.position) {
            out.push(issue(IssueCode::BadNumber, &[&n.id]));
        }
        if let Some(g) = &n.group_id
            && !groups.contains_key(g.as_str())
        {
            out.push(issue(IssueCode::UnknownGroup, &[&n.id, g]));
        }
        for (key, value) in &n.props {
            match spec.prop(key) {
                None => out.push(issue(IssueCode::UnknownProp, &[&n.id, key])),
                Some(p) if !p.type_matches(value) => {
                    out.push(issue(IssueCode::PropType, &[&n.id, key]));
                }
                Some(p) if !p.accepts(value) => {
                    out.push(issue(IssueCode::PropOutOfRange, &[&n.id, key]));
                }
                Some(_) => {}
            }
        }
    }

    // --- Notes ---
    for note in &design.notes {
        if chars(&note.text) > limits.max_note {
            out.push(issue(IssueCode::TextTooLong, &[&note.id]));
        }
        if !finite_point(&note.position) || !good_size(&note.size) {
            out.push(issue(IssueCode::BadNumber, &[&note.id]));
        }
    }

    // --- Edges ---
    let mut wires = HashSet::new();
    for e in &design.edges {
        let (Some(from), Some(to)) = (nodes.get(e.from.as_str()), nodes.get(e.to.as_str())) else {
            out.push(issue(IssueCode::UnknownNode, &[&e.id]));
            continue;
        };
        if e.from == e.to {
            out.push(issue(IssueCode::SelfLoop, &[&e.id]));
            continue;
        }
        if !wires.insert((e.from.as_str(), e.to.as_str(), e.protocol)) {
            out.push(issue(IssueCode::DuplicateEdge, &[&e.id]));
            continue;
        }
        if !registry
            .wiring
            .allowed(from.kind, to.kind)
            .contains(&e.protocol)
        {
            out.push(issue(IssueCode::WiringNotAllowed, &[&e.id, &e.from, &e.to]));
        }
    }

    out
}

/// The tooltip reason for a card-to-card wire, including the card-level checks the
/// kind table can't see. `None` means at least one protocol is still free for this pair.
pub fn wire_reason(
    design: &Design,
    registry: &Registry,
    from_id: &str,
    to_id: &str,
) -> Option<WiringReason> {
    if from_id == to_id {
        return Some(WiringReason::SelfLoop);
    }
    let from = design.nodes.iter().find(|n| n.id == from_id)?;
    let to = design.nodes.iter().find(|n| n.id == to_id)?;
    let allowed = registry.wiring.allowed(from.kind, to.kind);
    if allowed.is_empty() {
        return Some(registry.wiring.reason(from.kind, to.kind));
    }
    let used = |p| {
        design
            .edges
            .iter()
            .any(|e| e.from == from_id && e.to == to_id && e.protocol == p)
    };
    allowed
        .iter()
        .all(|p| used(*p))
        .then_some(WiringReason::DuplicateEdge)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::design::{Edge, Group, GroupKind, Node, Note, PropValue, Protocol};
    use crate::registry::{ComponentKind, Flavour, registry};

    fn node(id: &str, kind: ComponentKind, flavour: Flavour) -> Node {
        Node {
            id: id.into(),
            kind,
            flavour,
            label: String::new(),
            position: Point { x: 0.0, y: 0.0 },
            props: BTreeMap::new(),
            group_id: None,
        }
    }

    fn edge(id: &str, from: &str, to: &str, protocol: Protocol) -> Edge {
        Edge {
            id: id.into(),
            from: from.into(),
            to: to.into(),
            protocol,
        }
    }

    fn valid() -> Design {
        let mut d = Design::empty("d1", "Shortener");
        d.groups.push(Group {
            id: "r1".into(),
            kind: GroupKind::Region,
            label: "us-east-1".into(),
            position: Point { x: -50.0, y: -50.0 },
            size: Size { w: 900.0, h: 600.0 },
            parent_id: None,
            scaling: None,
            region_tag: None,
            mesh: None,
        });
        d.groups.push(Group {
            id: "az1".into(),
            kind: GroupKind::Az,
            label: "AZ a".into(),
            position: Point { x: 0.0, y: 0.0 },
            size: Size { w: 400.0, h: 300.0 },
            parent_id: Some("r1".into()),
            scaling: None,
            region_tag: None,
            mesh: None,
        });
        d.nodes.push(node("c", ComponentKind::Client, Flavour::Web));
        let mut app = node("a", ComponentKind::App, Flavour::Go);
        app.props.insert("instances".into(), PropValue::Number(3.0));
        app.props
            .insert("service_ms".into(), PropValue::Number(4.0));
        app.group_id = Some("az1".into());
        d.nodes.push(app);
        d.nodes
            .push(node("db", ComponentKind::Sql, Flavour::Postgresql));
        d.edges.push(edge("e1", "c", "a", Protocol::Http));
        d.edges.push(edge("e2", "a", "db", Protocol::Query));
        d.notes.push(Note {
            id: "n1".into(),
            text: "Reads are 95% of traffic.".into(),
            position: Point { x: 10.0, y: 400.0 },
            size: Size { w: 200.0, h: 120.0 },
        });
        d
    }

    fn codes(d: &Design) -> Vec<IssueCode> {
        validate(d, &registry())
            .into_iter()
            .map(|i| i.code)
            .collect()
    }

    #[test]
    fn a_valid_design_has_no_issues() {
        assert_eq!(validate(&valid(), &registry()), vec![]);
    }

    #[test]
    fn bad_and_duplicate_ids() {
        let mut d = valid();
        d.nodes[0].id = "has space".into();
        assert!(codes(&d).contains(&IssueCode::BadId));
        let mut d = valid();
        d.notes[0].id = "a".into();
        assert_eq!(codes(&d), vec![IssueCode::DuplicateId]);
    }

    #[test]
    fn edges_need_both_cards_and_an_allowed_protocol() {
        let mut d = valid();
        d.edges.push(edge("e3", "a", "ghost", Protocol::Http));
        assert_eq!(codes(&d), vec![IssueCode::UnknownNode]);
        let mut d = valid();
        d.edges.push(edge("e3", "c", "db", Protocol::Query));
        assert_eq!(codes(&d), vec![IssueCode::WiringNotAllowed]);
        let mut d = valid();
        d.edges.push(edge("e3", "a", "a", Protocol::Http));
        assert_eq!(codes(&d), vec![IssueCode::SelfLoop]);
        let mut d = valid();
        d.edges.push(edge("e3", "a", "db", Protocol::Query));
        assert_eq!(codes(&d), vec![IssueCode::DuplicateEdge]);
    }

    #[test]
    fn flavours_and_props_follow_the_registry() {
        let mut d = valid();
        d.nodes[1].flavour = Flavour::Redis;
        assert_eq!(codes(&d), vec![IssueCode::UnknownFlavour]);
        let mut d = valid();
        d.nodes[1]
            .props
            .insert("colour".into(), PropValue::Text("red".into()));
        assert_eq!(codes(&d), vec![IssueCode::UnknownProp]);
        let mut d = valid();
        d.nodes[1]
            .props
            .insert("instances".into(), PropValue::Bool(true));
        assert_eq!(codes(&d), vec![IssueCode::PropType]);
        let mut d = valid();
        d.nodes[1]
            .props
            .insert("instances".into(), PropValue::Number(999.0));
        assert_eq!(codes(&d), vec![IssueCode::PropOutOfRange]);
    }

    #[test]
    fn int_props_refuse_fractions_float_props_take_whole_numbers() {
        let mut d = valid();
        d.nodes[1]
            .props
            .insert("instances".into(), PropValue::Number(2.5));
        assert_eq!(codes(&d), vec![IssueCode::PropOutOfRange]);
        let mut d = valid();
        // service_ms is a float prop; a whole number from the UI is fine.
        d.nodes[1]
            .props
            .insert("service_ms".into(), PropValue::Number(2.0));
        assert_eq!(codes(&d), vec![]);
    }

    #[test]
    fn zones_must_exist_and_not_loop() {
        let mut d = valid();
        d.nodes[1].group_id = Some("nowhere".into());
        assert_eq!(codes(&d), vec![IssueCode::UnknownGroup]);
        let mut d = valid();
        d.groups[0].parent_id = Some("az1".into());
        assert_eq!(
            codes(&d),
            vec![IssueCode::GroupCycle, IssueCode::GroupCycle]
        );
    }

    #[test]
    fn text_and_numbers_are_bounded() {
        let mut d = valid();
        d.notes[0].text = "x".repeat(4001);
        assert_eq!(codes(&d), vec![IssueCode::TextTooLong]);
        let mut d = valid();
        d.nodes[0].position.x = f64::NAN;
        assert_eq!(codes(&d), vec![IssueCode::BadNumber]);
        let mut d = valid();
        d.groups[1].size.w = 0.0;
        assert_eq!(codes(&d), vec![IssueCode::BadNumber]);
        let mut d = valid();
        d.viewport.zoom = 0.0;
        assert_eq!(codes(&d), vec![IssueCode::BadNumber]);
    }

    fn autoscale_group(scaling: Option<crate::design::Scaling>) -> Group {
        Group {
            id: "asg".into(),
            kind: GroupKind::Autoscale,
            label: String::new(),
            position: Point { x: 0.0, y: 0.0 },
            size: Size { w: 300.0, h: 200.0 },
            parent_id: Some("az1".into()),
            scaling,
            region_tag: None,
            mesh: None,
        }
    }

    fn policy(flavour: Flavour, min: f64) -> crate::design::Scaling {
        crate::design::Scaling {
            flavour,
            props: BTreeMap::from([("min_instances".to_owned(), PropValue::Number(min))]),
        }
    }

    #[test]
    fn autoscale_wrappers_need_a_valid_policy() {
        let mut d = valid();
        d.groups
            .push(autoscale_group(Some(policy(Flavour::KubernetesHpa, 2.0))));
        d.nodes[1].group_id = Some("asg".into());
        assert_eq!(codes(&d), vec![]);
        for bad in [
            None,
            Some(policy(Flavour::Redis, 2.0)),
            Some(policy(Flavour::AwsAsg, 0.5)),
        ] {
            let mut d = valid();
            d.groups.push(autoscale_group(bad));
            assert_eq!(codes(&d), vec![IssueCode::BadScaling]);
        }
        // Only autoscale wrappers carry a policy.
        let mut d = valid();
        d.groups[0].scaling = Some(policy(Flavour::AwsAsg, 2.0));
        assert_eq!(codes(&d), vec![IssueCode::BadScaling]);
    }

    #[test]
    fn autoscaling_is_never_a_card() {
        let mut d = valid();
        d.nodes.push(node(
            "hpa",
            ComponentKind::Autoscale,
            Flavour::KubernetesHpa,
        ));
        assert_eq!(codes(&d), vec![IssueCode::WrapperAsCard]);
    }

    #[test]
    fn too_many_items() {
        let mut d = valid();
        for i in 0..200 {
            d.notes.push(Note {
                id: format!("extra{i}"),
                text: String::new(),
                position: Point { x: 0.0, y: 0.0 },
                size: Size { w: 1.0, h: 1.0 },
            });
        }
        assert_eq!(codes(&d), vec![IssueCode::TooManyItems]);
    }

    #[test]
    fn wire_reasons_cover_card_level_checks() {
        let d = valid();
        let r = registry();
        assert_eq!(wire_reason(&d, &r, "a", "a"), Some(WiringReason::SelfLoop));
        assert_eq!(
            wire_reason(&d, &r, "c", "db"),
            Some(WiringReason::ClientToData)
        );
        assert_eq!(
            wire_reason(&d, &r, "a", "db"),
            Some(WiringReason::DuplicateEdge)
        );
        assert_eq!(
            wire_reason(&d, &r, "c", "a"),
            None,
            "gRPC and WebSocket remain"
        );
    }

    /// v4: a residency tag sits on a Region only, as 2–8 lowercase letters; a mesh sits on a
    /// Region or AZ, with known props in range.
    #[test]
    fn zone_settings_are_checked() {
        use crate::design::{Mesh, MeshFlavour};
        let r = registry();
        let mut d = valid();
        d.groups[0].region_tag = Some("eu".into());
        let mesh = |k: &str, v: f64| Mesh {
            flavour: MeshFlavour::Istio,
            props: [(k.to_owned(), PropValue::Number(v))].into_iter().collect(),
        };
        d.groups[1].mesh = Some(mesh("mesh_retries", 2.0));
        assert!(validate(&d, &r).is_empty());
        for bad in ["EU", "e", "europe-west"] {
            let mut d = d.clone();
            d.groups[0].region_tag = Some(bad.into());
            assert_eq!(codes(&d), vec![IssueCode::BadZoneSetting], "{bad}");
        }
        let mut az_tag = d.clone();
        az_tag.groups[1].region_tag = Some("eu".into());
        assert_eq!(codes(&az_tag), vec![IssueCode::BadZoneSetting]);
        for (k, v) in [("mesh_retries", 11.0), ("retries", 1.0)] {
            let mut bad = d.clone();
            bad.groups[1].mesh = Some(mesh(k, v));
            assert_eq!(codes(&bad), vec![IssueCode::BadZoneSetting], "{k}");
        }
    }
}
