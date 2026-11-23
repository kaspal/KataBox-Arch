//! Route policies (ADR 0006): which component roles a request type must visit, in order.
//!
//! A route is a list of steps. A step is one or more alternatives joined by `|`, each a
//! role with optional `:qualifiers`, and the whole step may end in `?` (optional):
//!
//! ```text
//! "edge?"                 any number of edge hops (LB, CDN, gateway…), possibly none
//! "compute"               the entry service: it becomes the caller
//! "cache?"                cache-aside: a hit ends the request, a miss falls through
//! "datastore:write"       the data cluster's primary
//! "datastore:object"      a data store of that kind
//! "broker"                publish; the consumer after it becomes the caller
//! "cdc"                   change data capture: the database written by the step before
//!                         publishes the change to the stream on its CDC edge, and the
//!                         consumer after it becomes the caller
//! "external:email"        a provider outside the design
//! "lock"                  take a lock (lease) before writing: from a Coordination Service
//!                         (quorum, S7) or a cache used as a lock (no quorum)
//! "datastore:write:atomic" a conditional write (SQL constraint, compare-and-set): a race
//!                         on a contended key is refused cleanly instead of a conflict
//! "auth" "auth:login"     a token check / a login at an identity provider (M8)
//! "service"               a call to another service that answers without calling onward
//!                         (M8: a leaf dependency such as a recommendations service)
//! "compute|datastore"     either role satisfies the step
//! "broker?" "worker"      an optional hand-off: a caller wired to such a broker publishes
//!                         as usual; one that isn't makes the next segment's calls itself,
//!                         inline, so its answer waits for them
//! ```
//!
//! **CDC.** A `cdc` step must follow a `datastore:write` call to SQL or NoSQL. The caller's
//! answer leaves at the commit (the request's latency ends there); the change is published
//! by the database, not the caller, so it counts only toward completion. A stream that is
//! down delays changes (they wait in the database log, up to the store's `cdc_buffer_s`)
//! rather than losing them, and changes to one key keep their order.
//!
//! **The caller rule.** Steps before the first caller step (`compute`, `worker`, or an
//! alternation that includes one) are hops. From the first caller step on, the steps are
//! *calls made by that caller, in order*, until a `broker` step: the caller publishes, and
//! the consumer named by the next step (on the broker's Consume edge) becomes the caller of
//! the steps after it. Data stores never call onward.

use std::fmt;

use ka_core::ComponentKind;

/// What a node does on a request's path, from its kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Role {
    Edge,
    Compute,
    Worker,
    Cache,
    Datastore,
    Broker,
    External,
    /// M8 (S2): model inference, called as a leaf dependency.
    Model,
    /// M8: an identity provider (`auth` token checks, `auth:login` logins).
    Auth,
    Source,
    /// Never on a path (observability, autoscaling).
    None,
}

impl Role {
    pub fn of(kind: ComponentKind) -> Role {
        use ComponentKind as K;
        match kind {
            K::Dns | K::Cdn | K::Waf | K::Lb | K::Gateway | K::Ratelimit | K::Ws => Role::Edge,
            K::App | K::Micro | K::Serverless | K::Media => Role::Compute,
            K::Worker | K::Batch | K::Streamproc | K::Connector => Role::Worker,
            K::Cache => Role::Cache,
            K::Sql
            | K::Nosql
            | K::Object
            | K::Search
            | K::Coord
            | K::Vector
            | K::Warehouse
            | K::Lakehouse
            | K::Tsdb
            | K::Graph => Role::Datastore,
            K::Queue | K::Stream | K::Pubsub | K::Dlq | K::Mqtt => Role::Broker,
            K::Client | K::Scheduler => Role::Source,
            K::Infer => Role::Model,
            K::Idp => Role::Auth,
            // M8: pooled database connections, third-party API cards and config services take
            // part in requests through their own features (they are never a step's target).
            K::Dbproxy | K::External | K::Config => Role::None,
            K::Autoscale | K::Metrics | K::Logs | K::Tracing => Role::None,
        }
    }

    fn parse(s: &str) -> Option<Role> {
        Some(match s {
            "edge" => Role::Edge,
            "compute" => Role::Compute,
            "worker" => Role::Worker,
            "cache" => Role::Cache,
            "datastore" => Role::Datastore,
            "broker" => Role::Broker,
            "external" => Role::External,
            "model" => Role::Model,
            "auth" => Role::Auth,
            _ => return None,
        })
    }

    /// Roles that take over as the caller.
    pub fn is_caller(self) -> bool {
        matches!(self, Role::Compute | Role::Worker)
    }
}

/// One alternative of a step: a role and its qualifiers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Alt {
    pub role: Role,
    /// Writes go to the data cluster's primary (or fill a cache as the store).
    pub write: bool,
    /// The hop leaving the client must be a WebSocket.
    pub ws: bool,
    /// Restrict to these kinds (e.g. `datastore:object`).
    pub kinds: Vec<ComponentKind>,
    /// For `external:<name>`.
    pub external: Option<String>,
    /// A `service` step: a compute card called as a dependency (never a caller).
    pub service: bool,
    /// `:atomic`: a conditional write (S7).
    pub atomic: bool,
    /// `model:chat|embed|classify` (S2): only inference cards of that model type.
    pub model_type: Option<String>,
    /// `auth:login`: a login, not a token check.
    pub login: bool,
}

impl Alt {
    /// Does a node of `kind` satisfy this alternative?
    pub fn accepts(&self, kind: ComponentKind) -> bool {
        Role::of(kind) == self.role && (self.kinds.is_empty() || self.kinds.contains(&kind))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    pub alts: Vec<Alt>,
    pub optional: bool,
    /// The `cdc` step: a stream fed by the database written just before.
    pub cdc: bool,
    /// The `lock` step (S7).
    pub lock: bool,
}

impl Step {
    pub fn has_role(&self, role: Role) -> bool {
        self.alts.iter().any(|a| a.role == role)
    }

    pub fn accepts(&self, kind: ComponentKind) -> bool {
        self.alts.iter().any(|a| a.accepts(kind))
    }

    pub fn is_caller_step(&self) -> bool {
        self.alts.iter().any(|a| a.role.is_caller() && !a.service)
    }

    /// A cache step without `?` and without `:write` would be a read from a cache as the
    /// store; with `?` it is cache-aside. `:write` writes the cache.
    pub fn is_cache_aside(&self) -> bool {
        self.optional && self.has_role(Role::Cache) && !self.alts.iter().any(|a| a.write)
    }

    pub fn writes(&self) -> bool {
        self.alts.iter().any(|a| a.write)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteError(pub String);

impl fmt::Display for RouteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A parsed route, split by the caller rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Route {
    /// Edge hops before the entry: allowed at all, required (at least one), WebSocket.
    pub hops_allowed: bool,
    pub hops_required: bool,
    pub ws: bool,
    /// The entry step (the first caller, or `compute|datastore` behind a CDN). None for
    /// scheduler-sourced routes, whose scheduler is the first caller.
    pub entry: Option<Step>,
    /// Segment 0: the entry's calls; each later segment starts after a broker step.
    pub segments: Vec<Segment>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Segment {
    /// For segments after the first: the broker published to, and the consumer step.
    pub broker: Option<Step>,
    pub consumer: Option<Step>,
    /// Calls made by this segment's caller, in order.
    pub calls: Vec<Step>,
}

pub fn parse_step(s: &str) -> Result<Step, RouteError> {
    let (body, optional) = match s.strip_suffix('?') {
        Some(b) => (b, true),
        None => (s, false),
    };
    if body == "cdc" {
        if optional {
            return Err(RouteError("a cdc step can't be optional".into()));
        }
        return Ok(Step {
            alts: vec![Alt {
                role: Role::Broker,
                write: false,
                ws: false,
                kinds: vec![ComponentKind::Stream],
                external: None,
                service: false,
                atomic: false,
                model_type: None,
                login: false,
            }],
            optional: false,
            cdc: true,
            lock: false,
        });
    }
    if body == "lock" {
        let alt = |role: Role, kinds: Vec<ComponentKind>| Alt {
            role,
            write: true,
            ws: false,
            kinds,
            external: None,
            service: false,
            atomic: false,
            model_type: None,
            login: false,
        };
        return Ok(Step {
            alts: vec![
                alt(Role::Datastore, vec![ComponentKind::Coord]),
                alt(Role::Cache, Vec::new()),
            ],
            optional,
            cdc: false,
            lock: true,
        });
    }
    let mut alts = Vec::new();
    for alt in body.split('|') {
        let mut parts = alt.split(':');
        let role_s = parts.next().unwrap_or_default().trim();
        let service = role_s == "service";
        let role = if service {
            Role::Compute
        } else {
            Role::parse(role_s)
                .ok_or_else(|| RouteError(format!("unknown role {role_s:?} in {s:?}")))?
        };
        let mut a = Alt {
            role,
            write: false,
            ws: false,
            kinds: Vec::new(),
            external: None,
            service,
            atomic: false,
            model_type: None,
            login: false,
        };
        for q in parts {
            let q = q.trim();
            match q {
                "write" => a.write = true,
                "ws" => a.ws = true,
                "atomic" if role == Role::Datastore => a.atomic = true,
                "login" if role == Role::Auth => a.login = true,
                "chat" | "embed" | "classify" if role == Role::Model => {
                    a.model_type = Some(q.to_owned());
                }
                _ if role == Role::External => a.external = Some(q.to_owned()),
                _ => {
                    let kind: ComponentKind = serde_json::from_value(serde_json::Value::from(q))
                        .map_err(|_| RouteError(format!("unknown qualifier {q:?} in {s:?}")))?;
                    if Role::of(kind) != role && !(service && Role::of(kind) == Role::Compute) {
                        return Err(RouteError(format!("{q:?} is not a {role_s} in {s:?}")));
                    }
                    a.kinds.push(kind);
                }
            }
        }
        if role == Role::External && a.external.is_none() {
            return Err(RouteError(format!("external needs a name: {s:?}")));
        }
        alts.push(a);
    }
    if alts.iter().any(|a| a.atomic && !a.write) {
        return Err(RouteError(format!("`atomic` qualifies a write: {s:?}")));
    }
    Ok(Step {
        alts,
        optional,
        cdc: false,
        lock: false,
    })
}

/// Parse a route by the caller rule. `scheduler`: the route starts at a scheduler card,
/// which is the first caller (no hops, no entry step).
pub fn parse_route(steps: &[String], scheduler: bool) -> Result<Route, RouteError> {
    let parsed: Vec<Step> = steps
        .iter()
        .map(|s| parse_step(s))
        .collect::<Result<_, _>>()?;
    let mut i = 0;
    let (mut hops_allowed, mut hops_required, mut ws) = (false, false, false);
    let mut entry = None;
    if !scheduler {
        while i < parsed.len() && parsed[i].has_role(Role::Edge) && !parsed[i].is_caller_step() {
            hops_allowed = true;
            hops_required |= !parsed[i].optional;
            ws |= parsed[i].alts.iter().any(|a| a.ws);
            i += 1;
        }
        let e = parsed
            .get(i)
            .ok_or_else(|| RouteError("the route has no entry step".into()))?;
        if e.optional {
            return Err(RouteError("the entry step can't be optional".into()));
        }
        ws |= e.alts.iter().any(|a| a.ws);
        entry = Some(e.clone());
        i += 1;
    }
    let mut segments = vec![Segment {
        broker: None,
        consumer: None,
        calls: Vec::new(),
    }];
    while i < parsed.len() {
        let s = &parsed[i];
        if s.has_role(Role::Edge) {
            return Err(RouteError("edge steps come before the entry".into()));
        }
        if s.cdc {
            let wrote = segments
                .last()
                .and_then(|g| g.calls.last())
                .is_some_and(|c| {
                    c.writes()
                        && c.alts.iter().all(|a| {
                            a.role == Role::Datastore
                                && (a.kinds.is_empty()
                                    || a.kinds.iter().all(|k| {
                                        matches!(k, ComponentKind::Sql | ComponentKind::Nosql)
                                    }))
                        })
                });
            if !wrote {
                return Err(RouteError(
                    "a cdc step must follow a datastore:write call (SQL or NoSQL)".into(),
                ));
            }
        }
        if s.has_role(Role::Broker) {
            if s.optional && s.cdc {
                return Err(RouteError("a cdc step can't be optional".into()));
            }
            let consumer = parsed
                .get(i + 1)
                .filter(|c| c.is_caller_step())
                .ok_or_else(|| {
                    RouteError(
                        "a broker step must be followed by its consumer (worker or compute)".into(),
                    )
                })?;
            segments.push(Segment {
                broker: Some(s.clone()),
                consumer: Some(consumer.clone()),
                calls: Vec::new(),
            });
            i += 2;
            continue;
        }
        if s.is_caller_step() {
            return Err(RouteError(
                "a second caller needs a broker in between (data stores don't call onward)".into(),
            ));
        }
        segments
            .last_mut()
            .expect("at least one segment")
            .calls
            .push(s.clone());
        i += 1;
    }
    Ok(Route {
        hops_allowed,
        hops_required,
        ws,
        entry,
        segments,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(steps: &[&str]) -> Result<Route, RouteError> {
        parse_route(
            &steps.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>(),
            false,
        )
    }

    #[test]
    fn steps_parse_roles_qualifiers_and_alternation() {
        let s = parse_step("compute|datastore:object").unwrap();
        assert_eq!(s.alts.len(), 2);
        assert!(s.accepts(ComponentKind::App) && s.accepts(ComponentKind::Object));
        assert!(!s.accepts(ComponentKind::Sql));
        assert!(parse_step("cache?").unwrap().is_cache_aside());
        assert!(!parse_step("cache:write").unwrap().is_cache_aside());
        assert_eq!(
            parse_step("external:email").unwrap().alts[0]
                .external
                .as_deref(),
            Some("email")
        );
        assert!(
            parse_step("datastore:queue").is_err(),
            "a queue isn't a datastore"
        );
        assert!(parse_step("teleport").is_err());
        assert!(parse_step("external").is_err());
    }

    #[test]
    fn the_caller_rule_splits_segments() {
        // News Feed's post: the app writes, then publishes; the worker fills the caches.
        let route = r(&[
            "edge?",
            "compute",
            "datastore:write",
            "broker",
            "worker",
            "cache:write",
        ])
        .unwrap();
        assert!(route.hops_allowed && !route.hops_required);
        assert!(route.entry.as_ref().unwrap().has_role(Role::Compute));
        assert_eq!(route.segments.len(), 2);
        assert_eq!(route.segments[0].calls.len(), 1);
        assert!(route.segments[0].calls[0].writes());
        assert!(
            route.segments[1]
                .consumer
                .as_ref()
                .unwrap()
                .has_role(Role::Worker)
        );
        assert_eq!(route.segments[1].calls.len(), 1);
    }

    #[test]
    fn hops_entry_and_ws() {
        let route = r(&["edge:ws", "compute", "broker", "compute"]).unwrap();
        assert!(route.hops_required && route.ws);
        let cdn = r(&["edge?", "compute|datastore:object"]).unwrap();
        assert!(cdn.entry.unwrap().accepts(ComponentKind::Object));
    }

    #[test]
    fn invalid_routes_are_refused() {
        assert!(r(&["edge?"]).is_err(), "no entry");
        assert!(r(&["compute", "edge"]).is_err(), "edge after the entry");
        assert!(
            r(&["compute", "cache", "compute"]).is_err(),
            "a cache can't call a compute"
        );
        assert!(
            r(&["compute", "broker"]).is_err(),
            "broker without a consumer"
        );
    }

    #[test]
    fn a_broker_step_may_be_optional() {
        let route = r(&[
            "compute",
            "datastore:write",
            "broker?",
            "worker",
            "external:email",
        ])
        .unwrap();
        assert_eq!(route.segments.len(), 2);
        assert!(route.segments[1].broker.as_ref().unwrap().optional);
    }

    /// M8: `service` calls a compute card as a leaf dependency; it isn't a second caller.
    #[test]
    fn a_service_step_is_a_call() {
        let route = r(&["edge?", "compute", "service?", "datastore:write"]).unwrap();
        assert_eq!(route.segments.len(), 1);
        assert_eq!(route.segments[0].calls.len(), 2);
        assert!(route.segments[0].calls[0].accepts(ComponentKind::Micro));
        assert!(route.segments[0].calls[0].optional);
        assert!(
            parse_step("service:micro")
                .unwrap()
                .accepts(ComponentKind::Micro)
        );
        assert!(
            !parse_step("service:micro")
                .unwrap()
                .accepts(ComponentKind::App)
        );
    }

    /// S7: `lock` reaches a coordination service or a cache; `atomic` marks a write.
    #[test]
    fn lock_and_atomic_steps() {
        let l = parse_step("lock").unwrap();
        assert!(l.lock && l.accepts(ComponentKind::Coord) && l.accepts(ComponentKind::Cache));
        assert!(!l.accepts(ComponentKind::Sql));
        assert!(parse_step("datastore:write:atomic").unwrap().alts[0].atomic);
        assert!(
            parse_step("datastore:atomic").is_err(),
            "atomic reads make no sense"
        );
    }

    #[test]
    fn scheduler_routes_start_with_calls() {
        let route = parse_route(
            &["broker".into(), "worker".into(), "datastore:write".into()],
            true,
        )
        .unwrap();
        assert!(route.entry.is_none());
        assert_eq!(route.segments.len(), 2);
        assert!(route.segments[0].calls.is_empty());
    }
}
