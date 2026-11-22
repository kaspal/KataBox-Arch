//! Wiring rules: which protocol may connect which kinds (ADR 0004).
//!
//! The rules are **data** ([`WiringTable`]), defined once here and sent to the UI inside
//! the registry. The UI runs the same tiny evaluator over the same table for instant
//! feedback while you drag; Rust re-validates on save. A parity test in the UI checks its
//! evaluator reproduces [`WiringTable::matrix`] exactly, so the two can't drift.

use serde::{Deserialize, Serialize};

use crate::design::Protocol;
use crate::registry::{Category, ComponentKind};

/// A set of kinds: any kind in `categories` or `kinds`, minus `except`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct KindSet {
    pub categories: Vec<Category>,
    pub kinds: Vec<ComponentKind>,
    pub except: Vec<ComponentKind>,
}

impl KindSet {
    pub fn contains(&self, kind: ComponentKind) -> bool {
        !self.except.contains(&kind)
            && (self.categories.contains(&kind.category()) || self.kinds.contains(&kind))
    }
}

/// `from → to` may use `protocol` (when `same_kind`, only between two cards of one kind).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct WiringRule {
    /// A stable name for docs and tests, e.g. `cdn_origin`.
    pub id: String,
    pub protocol: Protocol,
    pub from: KindSet,
    pub to: KindSet,
    pub same_kind: bool,
}

/// Why a pair with no allowed protocol is refused (the first matching denial wins).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct WiringDenial {
    pub from: KindSet,
    pub to: KindSet,
    pub reason: WiringReason,
}

/// Reason codes shown in the blocked-wiring tooltip (`wiring.reason.<code>`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum WiringReason {
    /// A card wired to itself.
    SelfLoop,
    /// The same protocol already connects these two cards in this direction.
    DuplicateEdge,
    /// Clients only start requests; nothing calls into a client.
    IntoClient,
    /// A client must go through a service to reach a data store.
    ClientToData,
    /// Observability tools only receive telemetry.
    FromOps,
    /// Data stores answer queries; they don't start calls.
    DataCallsOut,
    /// Messages leave a queue or stream only through a consumer service.
    ConsumeNeedsCompute,
    /// Put a service between the edge and the data.
    EdgeToData,
    /// No rule connects these kinds.
    NoRule,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct WiringTable {
    pub rules: Vec<WiringRule>,
    pub denials: Vec<WiringDenial>,
}

/// One cell of the kind × kind matrix: the allowed protocols, or why there are none.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct WiringCell {
    pub from: ComponentKind,
    pub to: ComponentKind,
    pub protocols: Vec<Protocol>,
    pub reason: Option<WiringReason>,
}

impl WiringTable {
    /// The protocols allowed from a `from` card to a `to` card, in `Protocol` order.
    pub fn allowed(&self, from: ComponentKind, to: ComponentKind) -> Vec<Protocol> {
        let mut out: Vec<Protocol> = self
            .rules
            .iter()
            .filter(|r| r.from.contains(from) && r.to.contains(to) && (!r.same_kind || from == to))
            .map(|r| r.protocol)
            .collect();
        out.sort();
        out.dedup();
        out
    }

    /// Why `from → to` has no protocol (only meaningful when [`Self::allowed`] is empty).
    pub fn reason(&self, from: ComponentKind, to: ComponentKind) -> WiringReason {
        self.denials
            .iter()
            .find(|d| d.from.contains(from) && d.to.contains(to))
            .map_or(WiringReason::NoRule, |d| d.reason)
    }

    /// Every kind pair, row by row in registry order.
    pub fn matrix(&self) -> Vec<WiringCell> {
        let mut out = Vec::with_capacity(ComponentKind::ALL.len().pow(2));
        for from in ComponentKind::ALL {
            for to in ComponentKind::ALL {
                let protocols = self.allowed(from, to);
                let reason = protocols.is_empty().then(|| self.reason(from, to));
                out.push(WiringCell {
                    from,
                    to,
                    protocols,
                    reason,
                });
            }
        }
        out
    }
}

// --- The table ----------------------------------------------------------------------

fn set(categories: &[Category], kinds: &[ComponentKind], except: &[ComponentKind]) -> KindSet {
    KindSet {
        categories: categories.to_vec(),
        kinds: kinds.to_vec(),
        except: except.to_vec(),
    }
}
fn cats(categories: &[Category]) -> KindSet {
    set(categories, &[], &[])
}
fn only(kinds: &[ComponentKind]) -> KindSet {
    set(&[], kinds, &[])
}
fn rule(id: &str, protocol: Protocol, from: KindSet, to: KindSet) -> WiringRule {
    WiringRule {
        id: id.into(),
        protocol,
        from,
        to,
        same_kind: false,
    }
}

/// The wiring table. Each rule names the pattern it allows.
pub fn table() -> WiringTable {
    use Category as C;
    use ComponentKind as K;
    use Protocol as P;

    // Compute that serves requests (Autoscaling is a controller, not a server).
    let services = || set(&[C::Compute], &[], &[K::Autoscale]);
    // Request callers and hops: clients, the edge, and services.
    let callers = || set(&[C::Edge, C::Compute], &[], &[K::Autoscale]);
    // Request targets: the edge (not clients) and services.
    let hops = || set(&[C::Edge, C::Compute], &[], &[K::Client, K::Autoscale]);
    let brokers = || only(&[K::Queue, K::Stream, K::Pubsub, K::Dlq]);

    let rules = vec![
        rule("sync_call", P::Http, callers(), hops()),
        rule("sync_call", P::Grpc, callers(), hops()),
        rule(
            "websocket",
            P::Ws,
            only(&[K::Client, K::Lb, K::Gateway]),
            only(&[K::Lb, K::Gateway, K::App, K::Micro]),
        ),
        // Services, search indexers and token-bucket limiters read and write data.
        rule(
            "query",
            P::Query,
            set(&[C::Compute], &[K::Search], &[K::Autoscale]),
            cats(&[C::Data]),
        ),
        rule(
            "token_bucket",
            P::Query,
            only(&[K::Ratelimit]),
            only(&[K::Cache]),
        ),
        // A CDN pulling from object storage as its origin (static sites, pastes).
        rule("cdn_origin", P::Http, only(&[K::Cdn]), only(&[K::Object])),
        // Presigned uploads straight to object storage: the one client → data exception.
        rule(
            "presigned_upload",
            P::Http,
            only(&[K::Client]),
            only(&[K::Object]),
        ),
        rule(
            "publish",
            P::Publish,
            set(&[C::Compute], &[K::Client, K::Scheduler], &[K::Autoscale]),
            only(&[K::Queue, K::Stream, K::Pubsub]),
        ),
        rule(
            "dead_letter",
            P::Publish,
            only(&[K::Queue, K::Stream]),
            only(&[K::Dlq]),
        ),
        // Change data capture and the outbox pattern.
        rule(
            "change_data_capture",
            P::Publish,
            only(&[K::Sql, K::Nosql]),
            only(&[K::Stream]),
        ),
        // Topic → queue fan-out (SNS → SQS).
        rule("fan_out", P::Publish, only(&[K::Pubsub]), only(&[K::Queue])),
        // Consume edges start at a broker and end at a service.
        rule("consume", P::Consume, brokers(), services()),
        // Replication between two stores of one kind; streams mirror across regions.
        WiringRule {
            id: "replicate".into(),
            protocol: P::Replicate,
            from: set(&[C::Data], &[K::Stream], &[]),
            to: set(&[C::Data], &[K::Stream], &[]),
            same_kind: true,
        },
        // A schedule triggering a job directly.
        rule("scheduled_job", P::Http, only(&[K::Scheduler]), services()),
        // Autoscaling is a wrapper around compute cards (v2), never wired: no rule.
        // Telemetry: anything but a client may ship metrics, logs and traces.
        rule(
            "telemetry",
            P::Http,
            set(&[C::Edge, C::Compute, C::Data, C::Async], &[], &[K::Client]),
            cats(&[C::Ops]),
        ),
    ];

    let all = || cats(&Category::ALL);
    let denials = vec![
        WiringDenial {
            from: all(),
            to: only(&[K::Client]),
            reason: WiringReason::IntoClient,
        },
        WiringDenial {
            from: only(&[K::Client]),
            to: cats(&[C::Data]),
            reason: WiringReason::ClientToData,
        },
        WiringDenial {
            from: cats(&[C::Ops]),
            to: all(),
            reason: WiringReason::FromOps,
        },
        WiringDenial {
            from: cats(&[C::Data]),
            to: all(),
            reason: WiringReason::DataCallsOut,
        },
        WiringDenial {
            from: brokers(),
            to: all(),
            reason: WiringReason::ConsumeNeedsCompute,
        },
        WiringDenial {
            from: cats(&[C::Edge]),
            to: cats(&[C::Data]),
            reason: WiringReason::EdgeToData,
        },
    ];
    WiringTable { rules, denials }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ComponentKind as K;
    use Protocol as P;

    fn t() -> WiringTable {
        table()
    }

    #[test]
    fn a_client_never_reaches_a_data_store_directly_except_object_storage() {
        for to in [K::Sql, K::Nosql, K::Cache, K::Search] {
            assert!(t().allowed(K::Client, to).is_empty(), "client → {to:?}");
            assert_eq!(t().reason(K::Client, to), WiringReason::ClientToData);
        }
        assert_eq!(t().allowed(K::Client, K::Object), vec![P::Http]);
    }

    #[test]
    fn consume_starts_at_a_broker_and_ends_at_a_service() {
        for from in ComponentKind::ALL {
            for to in ComponentKind::ALL {
                if t().allowed(from, to).contains(&P::Consume) {
                    assert!(
                        matches!(from, K::Queue | K::Stream | K::Pubsub | K::Dlq),
                        "{from:?}"
                    );
                    assert!(
                        matches!(to, K::App | K::Micro | K::Worker | K::Serverless),
                        "{to:?}"
                    );
                }
            }
        }
        assert_eq!(t().allowed(K::Queue, K::Worker), vec![P::Consume]);
        assert_eq!(
            t().reason(K::Queue, K::Sql),
            WiringReason::ConsumeNeedsCompute
        );
    }

    #[test]
    fn the_review_additions_are_allowed() {
        assert_eq!(t().allowed(K::Cdn, K::Object), vec![P::Http]);
        assert_eq!(t().allowed(K::Sql, K::Stream), vec![P::Publish]);
        assert_eq!(t().allowed(K::Nosql, K::Stream), vec![P::Publish]);
        assert_eq!(t().allowed(K::Scheduler, K::Worker), vec![P::Http]);
        assert!(t().allowed(K::Stream, K::Stream).contains(&P::Replicate));
    }

    #[test]
    fn common_paths() {
        assert_eq!(t().allowed(K::Client, K::Lb), vec![P::Http, P::Grpc, P::Ws]);
        assert_eq!(t().allowed(K::App, K::Sql), vec![P::Query]);
        assert_eq!(t().allowed(K::App, K::Queue), vec![P::Publish]);
        assert_eq!(t().allowed(K::Sql, K::Sql), vec![P::Replicate]);
        assert!(t().allowed(K::Sql, K::Nosql).is_empty());
        assert_eq!(t().allowed(K::App, K::Metrics), vec![P::Http]);
        assert_eq!(t().reason(K::Metrics, K::App), WiringReason::FromOps);
        assert_eq!(t().reason(K::App, K::Client), WiringReason::IntoClient);
        assert_eq!(t().reason(K::Lb, K::Sql), WiringReason::EdgeToData);
        assert_eq!(t().reason(K::Sql, K::App), WiringReason::DataCallsOut);
    }

    fn id<T: serde::Serialize>(v: &T) -> String {
        serde_json::to_value(v)
            .unwrap()
            .as_str()
            .unwrap()
            .to_owned()
    }

    /// The full matrix, pinned. Regenerate with `KA_UPDATE_SNAPSHOTS=1 cargo test -p ka-core`.
    #[test]
    fn matrix_snapshot() {
        let mut lines = Vec::new();
        for cell in t().matrix() {
            let rhs = if cell.protocols.is_empty() {
                format!("✗ {}", id(&cell.reason.unwrap()))
            } else {
                cell.protocols.iter().map(id).collect::<Vec<_>>().join(",")
            };
            lines.push(format!("{} -> {}: {rhs}", id(&cell.from), id(&cell.to)));
        }
        let got = lines.join("\n") + "\n";
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/src/wiring_matrix.snap");
        if std::env::var_os("KA_UPDATE_SNAPSHOTS").is_some() {
            std::fs::write(path, &got).unwrap();
        }
        let want = std::fs::read_to_string(path)
            .unwrap_or_default()
            .replace("\r\n", "\n");
        assert!(
            got == want,
            "wiring matrix changed; review it, then run with KA_UPDATE_SNAPSHOTS=1"
        );
    }
}
