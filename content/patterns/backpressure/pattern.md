Every stage has a limit on how much work it will hold: a maximum queue depth, a fixed pool of threads, a cap on requests in flight. When the limit is reached, the stage stops accepting work, and the stage before it has to wait, slow down or refuse its own callers.

The signal travels upstream until it reaches a place that can deal with it, usually the edge, where a request can be rejected with a clear "try again later". That is far better than accepting everything and letting latency climb for every user.

A queue in front of workers absorbs bursts, but only up to its bound. Size it for the bursts you expect to survive, watch its depth, and scale the workers on that depth so the queue drains before it fills.
