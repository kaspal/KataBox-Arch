## 1

Suppose you have a single gateway and it goes away. What happens to every conversation open on it, and to every message sent after that?

## 2

Run several gateways behind a load balancer, and join them with pub/sub. When every gateway subscribes to the same bus, any gateway can deliver a message, no matter which gateway it arrived at.

## 3

Put at least two gateway cards behind a Network Load Balancer that carries WebSockets. Have each card publish to and consume from one Pub/Sub bus, and give them enough instances for an evening much busier than normal. More instances inside one card do not help if the whole card stops.
