## 1

Both databases stayed up during the split, and reads kept working. Only writes failed. What does a commit wait for before it returns?

## 2

With `sync = sync` every commit waits for the one replica. When that replica is cut off or dead, no commit can finish, so losing either zone stops all writes. A commit needs an acknowledgement that survives losing any one zone.

## 3

Add a third PostgreSQL card in a new AZ-c, replicated from the primary, and set `sync = quorum` on every card of the cluster: a commit then waits for any one of the two replicas. Nothing else needs to change.
