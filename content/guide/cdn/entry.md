A content delivery network (CDN) is a set of caches spread around the world, at the **edge** of the network close to users. A request for a file goes to the nearest edge. If the edge holds a fresh copy (a **hit**), it answers in a few milliseconds. If not (a **miss**), it fetches the file from your **origin**, such as object storage or an app server, and keeps a copy for the next user.

It works best for content that is the same for everyone: images, scripts, style sheets, video segments. The origin then sees only the misses, so a CDN can turn a flood of reads into a trickle.

In the simulator a CDN has unlimited capacity and caches only request types the problem marks as cacheable; everything else passes through to the origin. Popular keys hit far more often than rare ones, and each copy expires after the TTL.
