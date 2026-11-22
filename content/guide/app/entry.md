An app server runs your code. It takes a request, checks it, calls whatever it needs (a cache, a database, another service) and builds the answer. Most designs keep it **stateless**, with nothing about a user kept in its memory, so any instance can serve any request and you scale by adding instances behind a load balancer.

Each instance can only work on so many requests at once, its **concurrency**. A request keeps its slot for its whole life, including the time spent waiting on the database. So a slow call further down fills the app tier even when the app itself has little to do.

In the simulator capacity is instances times concurrency. When every slot is busy, new requests wait in a short queue, and a full queue drops them. The flavours differ in their default service time and concurrency, which come from each runtime's usual setup.
