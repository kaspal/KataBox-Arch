A cache keeps a copy of frequently read data in memory, close to your app servers. A read that finds its key there (a **hit**) returns in well under a millisecond. A read that doesn't (a **miss**) falls through to the database, and the answer is stored in the cache on the way back, so the next read hits.

It trades a little freshness for a lot of speed, and it takes read load off the database, which is usually the hardest part of a system to scale.

How well it works depends on the traffic. When a few keys get most of the reads, a small cache catches most of them. When every key is equally likely, the cache has to be nearly as big as the data to help.
