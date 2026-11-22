## 1

Every request needs the session store, and there is nothing behind it. What happens to the site if that one card stops answering?

## 2

Keep the App Servers interchangeable and spread them over more than one card, so losing one still leaves enough. Then give the session store a second copy that can take over when the first one fails.

## 3

Put a Load Balancer in front of two Node.js App Server cards of 5 instances each. Make the session store a Redis Cache big enough for 2 million 2 KB sessions, with a TTL of a day, and wire a second Redis to it with a Replicate edge. When the primary dies, the replica is promoted after the failover time.
