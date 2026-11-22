//! KataBox Arch domain types. No I/O: everything here is plain data and pure functions,
//! shared by the app service (`ka-app`), and later the simulator and the judge.
//!
//! - [`design`]: what a `.kbarch` file holds (nodes, edges, zones, notes, viewport).
//! - [`registry`]: the 25 component kinds, their product flavours and editable props.
//! - [`wiring`]: which protocol may connect which kinds, as a data table the UI also gets.
//! - [`validate`]: checks a design against the registry and the wiring table.

pub mod design;
pub mod detmath;
pub mod problem;
pub mod registry;
pub mod rng;
pub mod scenario;
pub mod story;
pub mod validate;
pub mod wiring;

pub use design::{
    Design, Edge, Group, GroupKind, Node, Note, Point, PropValue, Protocol, SCHEMA_VERSION,
    Scaling, Size, Viewport,
};
pub use registry::{
    CALIBRATED_PROPS, CITED, CalibrationRef, CalibrationRow, CalibrationStatus, Category,
    ComponentKind, Cost, Flavour, FlavourSpec, KindSpec, MetaKind, MetaSpec, Placement,
    PropOverride, PropSpec, PropType, Registry, SimDefaults, Unit, calibration_rows, flavour_id,
    registry,
};
pub use validate::{DesignIssue, IssueCode, Limits, validate, wire_reason};
pub use wiring::{KindSet, WiringCell, WiringDenial, WiringReason, WiringRule, WiringTable};

/// Serializes a number the way JavaScript would print it: whole numbers without a
/// trailing `.0`, so `.kbarch` files read naturally and survive a trip through the UI.
pub(crate) fn ser_num<S: serde::Serializer>(v: &f64, s: S) -> Result<S::Ok, S::Error> {
    // 2^53: beyond it, not every integer is representable, so keep the float form.
    const MAX_SAFE: f64 = 9_007_199_254_740_992.0;
    if v.is_finite() && v.fract() == 0.0 && v.abs() < MAX_SAFE {
        #[allow(clippy::cast_possible_truncation)]
        s.serialize_i64(*v as i64)
    } else {
        s.serialize_f64(*v)
    }
}
