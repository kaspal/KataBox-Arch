## 1

The failover took about ten seconds and then the errors stopped, yet the test still fails. Look at the lost count: which payments were confirmed to customers but never made it to the new primary?

## 2

The primary confirmed each commit before any replica had it, and the replicas trail it by about 100 ms. When it died, the replica that took over had never seen its last 100 ms of payments. A commit that must not be lost has to wait until another member has it too.

## 3

Set `sync` to `quorum` on all three PostgreSQL cards, and keep them in three AZs. A commit then waits for a majority of the cluster, so any one member can die without losing a confirmed payment, and the two that remain are still a majority, so commits go on after the failover.
