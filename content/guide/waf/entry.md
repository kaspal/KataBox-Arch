A web application firewall (WAF) sits in front of your public endpoints and reads every HTTP request before it reaches your code. It checks each one against a set of **rules**: known attack patterns such as SQL injection or cross-site scripting, bad bots, blocked countries, or limits you write yourself. A request that matches a blocking rule is turned away at the edge.

Managed rule sets are kept up to date by the vendor, so you get protection against common attacks without changing your app. The price is a small delay on every request, a per-request bill, and the risk that a rule catches harmless traffic.

In the simulator, a WAF is a hop with capacity and a short rule-check time on every request. Attacks are not simulated, so it costs time and money but does not change who gets through. To shed a flood from one key, use a Rate Limiter.
