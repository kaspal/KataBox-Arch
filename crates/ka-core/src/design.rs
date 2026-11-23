//! The design: what a `.kbarch` file holds (ADR 0003). Every struct refuses unknown
//! fields, so a file from a newer app, or a hand-edited typo, is reported instead of
//! silently dropped.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::registry::{ComponentKind, Flavour};
use crate::ser_num;

/// The `.kbarch` format version this build writes. Older versions are migrated on open;
/// newer ones are refused. v3: cards store only the props the user set (ADR 0003). v4 (M8):
/// zones may carry a residency `region_tag` and a service-mesh setting.
pub const SCHEMA_VERSION: u32 = 4;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(deny_unknown_fields)]
pub struct Design {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    /// The problem this design solves; `None` in the Sandbox.
    pub problem_id: Option<String>,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    /// Region and AZ zones (drawn behind the cards).
    pub groups: Vec<Group>,
    pub notes: Vec<Note>,
    pub viewport: Viewport,
}

impl Design {
    /// An empty design with the current schema version.
    pub fn empty(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            id: id.into(),
            name: name.into(),
            problem_id: None,
            nodes: Vec::new(),
            edges: Vec::new(),
            groups: Vec::new(),
            notes: Vec::new(),
            viewport: Viewport {
                x: 0.0,
                y: 0.0,
                zoom: 1.0,
            },
        }
    }
}

/// A component card. `position` is the card's top-left corner in canvas coordinates,
/// always absolute (never relative to its zone).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(deny_unknown_fields)]
pub struct Node {
    pub id: String,
    pub kind: ComponentKind,
    pub flavour: Flavour,
    pub label: String,
    pub position: Point,
    /// Editable props by key (see the kind's `PropSpec`s). Missing keys take the default.
    pub props: BTreeMap<String, PropValue>,
    /// The innermost zone holding the card, if any.
    pub group_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(deny_unknown_fields)]
pub struct Edge {
    pub id: String,
    pub from: String,
    pub to: String,
    pub protocol: Protocol,
}

/// What travels along an edge. It sets per-hop latency (M1) and the drawing style:
/// sync calls solid, async messages dashed, replication dotted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum Protocol {
    Http,
    Grpc,
    Ws,
    Query,
    Publish,
    Consume,
    Replicate,
}

impl Protocol {
    pub const ALL: [Protocol; 7] = [
        Protocol::Http,
        Protocol::Grpc,
        Protocol::Ws,
        Protocol::Query,
        Protocol::Publish,
        Protocol::Consume,
        Protocol::Replicate,
    ];
}

/// A Region or AZ frame. Membership is by `Node::group_id` / `Group::parent_id`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(deny_unknown_fields)]
pub struct Group {
    pub id: String,
    pub kind: GroupKind,
    pub label: String,
    pub position: Point,
    pub size: Size,
    /// The zone this one sits in: an AZ's region, an autoscaling group's AZ or region.
    pub parent_id: Option<String>,
    /// The scaling policy of an Autoscale wrapper (and only of one).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scaling: Option<Scaling>,
    /// A Region's residency tag (v4, S9), e.g. `eu`: requests that must stay in a region
    /// check the tags of the stores they touch.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region_tag: Option<String>,
    /// A service mesh over the cards in this Region or AZ (v4): sidecar latency and the
    /// mesh's retries, timeouts and circuit breaking.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mesh: Option<Mesh>,
}

/// A zone's service mesh (v4): the product and its policy props (`registry.mesh_props`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(deny_unknown_fields)]
pub struct Mesh {
    pub flavour: MeshFlavour,
    pub props: BTreeMap<String, PropValue>,
}

/// Service-mesh products.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum MeshFlavour {
    Istio,
    Linkerd,
}

/// An autoscaling group's policy: the Autoscaling kind's flavour and props (v2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(deny_unknown_fields)]
pub struct Scaling {
    pub flavour: Flavour,
    pub props: BTreeMap<String, PropValue>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum GroupKind {
    Region,
    Az,
    /// An autoscaling group wrapped around compute cards (v2).
    Autoscale,
}

/// A sticky note (hand-written font on the canvas).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(deny_unknown_fields)]
pub struct Note {
    pub id: String,
    pub text: String,
    pub position: Point,
    pub size: Size,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(deny_unknown_fields)]
pub struct Point {
    #[serde(serialize_with = "ser_num")]
    pub x: f64,
    #[serde(serialize_with = "ser_num")]
    pub y: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(deny_unknown_fields)]
pub struct Size {
    #[serde(serialize_with = "ser_num")]
    pub w: f64,
    #[serde(serialize_with = "ser_num")]
    pub h: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(deny_unknown_fields)]
pub struct Viewport {
    #[serde(serialize_with = "ser_num")]
    pub x: f64,
    #[serde(serialize_with = "ser_num")]
    pub y: f64,
    #[serde(serialize_with = "ser_num")]
    pub zoom: f64,
}

/// A prop value. Numbers are one type: JavaScript can't tell `2` from `2.0`, so an
/// integer prop is a number that the registry checks is whole (`PropType::Int`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(untagged)]
pub enum PropValue {
    Number(#[serde(serialize_with = "ser_num")] f64),
    Bool(bool),
    Text(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whole_numbers_serialize_without_a_fraction() {
        let p = Point { x: 120.0, y: -8.5 };
        assert_eq!(serde_json::to_string(&p).unwrap(), r#"{"x":120,"y":-8.5}"#);
        let v = PropValue::Number(2.0);
        assert_eq!(serde_json::to_string(&v).unwrap(), "2");
    }

    #[test]
    fn prop_values_parse_from_plain_json() {
        let v: PropValue = serde_json::from_str("2").unwrap();
        assert_eq!(v, PropValue::Number(2.0));
        let v: PropValue = serde_json::from_str("2.5").unwrap();
        assert_eq!(v, PropValue::Number(2.5));
        let v: PropValue = serde_json::from_str("true").unwrap();
        assert_eq!(v, PropValue::Bool(true));
        let v: PropValue = serde_json::from_str(r#""lru""#).unwrap();
        assert_eq!(v, PropValue::Text("lru".into()));
    }

    #[test]
    fn unknown_fields_are_refused() {
        let err = serde_json::from_str::<Point>(r#"{"x":1,"y":2,"z":3}"#).unwrap_err();
        assert!(err.to_string().contains("unknown field"), "{err}");
    }
}
