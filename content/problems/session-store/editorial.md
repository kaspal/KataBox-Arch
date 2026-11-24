# Editorial

## Where the load goes

At 20,000 requests/s, about 18,000 are page loads that read a session and about 2,000 are sign-ins that write one. 2 million sessions of about 2 KB is roughly 4 GB, so one Redis of 8 GB holds them all with room to spare. Redis answers in a fraction of a millisecond, so the time goes to the App Servers: 20,000 requests × 10 ms ≈ 200 slots busy at once.

Speed is easy. The hard part is that every request depends on the session store.

## The reference design

A Load Balancer (AWS ALB, round robin) spreads requests over two Node.js App Server cards, Web A and Web B, with 5 instances each at the default concurrency of 64: 640 slots, 320 per card. Both read and write sessions in a Redis Cache of 8 GB with a TTL of 86,400 s, so a session lasts a day. A second Redis of the same size is wired to it with a Replicate edge. This costs about $920 per month, with a p95 near 31 ms.

"An app server dies" kills Web A at 20 seconds, for good. It allows 3% errors, 97% availability and a p95 of 100 ms. The Load Balancer keeps sending half the requests to the dead card until health checks notice it, 2 to 4 seconds later, and those requests fail: about 1.7% of the run. After that, Web B's 320 slots carry the whole load.

"The session store's primary fails" kills the Redis primary at 20 seconds, for good, in a 120-second run, and allows 5% errors. Reads already spread over both copies, so once health checks steer page loads away from the dead primary, the replica answers them. Sign-ins fail until the replica is promoted, after Redis's default failover time of 5 seconds. About 1.2% of requests fail in all.

## Why the first try breaks

A common first try uses the same two App Server cards, but the Load Balancer routes by key hash, so each user sticks to one card, and there is a single Redis with no replica. It costs about $823 per month and passes "Normal day".

It also passes "An app server dies", with about 1.7% errors, the same as the reference: sticky routing does not help or hurt here, because the sessions live in Redis either way. The failover test is where it breaks. With no replica there is nothing to promote, so from 20 seconds to the end of the run every request fails. That is 100 of 120 seconds: about 83% errors.

The lesson: moving sessions out of the App Servers only moves the single point of failure, unless the store itself has a copy that can take over.

## What the simulator leaves out

The classic first try keeps sessions in each App Server's memory and uses sticky routing so a user keeps landing on the server that holds theirs. When that server dies, its users are signed out. The simulator does not model state inside App Servers, so it cannot show that failure. This problem tests the part it can measure: the session store and its failover.

## Things to try

- Delete the replica and run "The session store's primary fails". Then put it back and lower the failover time.
- Merge the two App Server cards into one of 10 instances and run "An app server dies".
- Switch the reference's Load Balancer to key hash and run both hidden tests. The numbers barely move.
