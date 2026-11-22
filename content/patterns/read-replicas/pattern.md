The primary takes every write and streams its changes to one or more replicas. Each replica applies them in order and keeps a full copy of the data. The app sends writes to the primary and spreads reads over all the healthy copies.

Replication takes a moment, so a replica is always a little behind. A user who writes and then reads straight away may not see their own change. Where that matters, read that user's data from the primary for a short while after they write.

Replicas also make failover possible. When the primary dies, a replica is promoted and takes the writes. Until the promotion finishes, writes fail, so the failover time is part of your availability budget.
