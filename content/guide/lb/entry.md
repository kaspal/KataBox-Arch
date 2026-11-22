A load balancer gives many servers one front door. Clients connect to the balancer, and it forwards each request to one of the servers behind it, following an **algorithm** such as taking turns or picking the least busy. To grow, you add servers behind it; the clients never notice.

It also runs **health checks**: it asks each server, every few seconds, whether it is alive. A server that fails a couple of checks in a row is taken out of rotation, and one that recovers is put back. Until a failure is noticed, requests sent to the dead server fail.

In the simulator the balancer adds a short proxying time and has plenty of capacity. Its real job is spreading load and steering around dead cards, so the servers behind it decide how much traffic the design can take.
