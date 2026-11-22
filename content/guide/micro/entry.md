A **microservice** is a small, separately deployed service that owns one part of the business, such as payments or user profiles, together with that part's data. Other services reach it over the network, through its API or through events it publishes.

The point is independence. Each service can be released, scaled and fixed without touching the others, so teams move faster and one busy feature can grow on its own. The price is that calls that used to be function calls now cross the network: they take longer, they can fail, and you need tracing to follow a request through them.

In the simulator a Microservice card behaves like an App Server: a pool of instances, each working on a fixed number of requests at once, and each request keeps its slot until the calls it makes have answered. It can fill any compute step of a request's route.
