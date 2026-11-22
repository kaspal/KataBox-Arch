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
//! "external:email"        a provider outside the design
//! "compute|datastore"     either role satisfies the step
//! ```
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
    Source,
    /// Never on a path (observability, autoscaling).
    None,
}

impl Role {
    pub fn of(kind: ComponentKind) -> Role {
        use ComponentKind as K;
        match kind {
            K::Dns | K::Cdn | K::Waf | K::Lb | K::Gateway | K::Ratelimit => Role::Edge,
            K::App | K::Micro | K::Serverless => Role::Compute,
            K::Worker => Role::Worker,
            K::Cache => Role::Cache,
            K::Sql | K::Nosql | K::Object | K::Search => Role::Datastore,
            K::Queue | K::Stream | K::Pubsub | K::Dlq => Role::Broker,
            K::Client | K::Scheduler => Role::Source,
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
}

impl Step {
    pub fn has_role(&self, role: Role) -> bool {
        self.alts.iter().any(|a| a.role == role)
    }

    pub fn accepts(&self, kind: ComponentKind) -> bool {
        self.alts.iter().any(|a| a.accepts(kind))
    }

    pub fn is_caller_step(&self) -> bool {
        self.alts.iter().any(|a| a.role.is_caller())
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
    let mut alts = Vec::new();
    for alt in body.split('|') {
        let mut parts = alt.split(':');
        let role_s = parts.next().unwrap_or_default().trim();
        let role = Role::parse(role_s)
            .ok_or_else(|| RouteError(format!("unknown role {role_s:?} in {s:?}")))?;
        let mut a = Alt {
            role,
            write: false,
            ws: false,
            kinds: Vec::new(),
            external: None,
        };
        for q in parts {
            let q = q.trim();
            match q {
                "write" => a.write = true,
                "ws" => a.ws = true,
                _ if role == Role::External => a.external = Some(q.to_owned()),
                _ => {
                    let kind: ComponentKind = serde_json::from_value(serde_json::Value::from(q))
                        .map_err(|_| RouteError(format!("unknown qualifier {q:?} in {s:?}")))?;
                    if Role::of(kind) != role {
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
    Ok(Step { alts, optional })
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
        if s.has_role(Role::Broker) {
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
