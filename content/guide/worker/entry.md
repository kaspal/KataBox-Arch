A **worker** is a background process that takes jobs from a queue or a stream and runs them, away from the user's request. The web tier puts a job on the queue, answers "accepted" straight away, and the worker does the slow part later: transcoding a video, sending an email, judging a submission.

This splits two kinds of load. The request path stays fast and cheap, and the slow work gets its own pool that you size for throughput rather than response time. During a burst the queue holds the extra jobs and the workers catch up afterwards, so a busy minute means late results instead of errors.

In the simulator a Worker card is the consumer on a broker's Consume edge. Each instance runs a fixed number of jobs at once, any free slot takes the next message, and a job holds its slot until the calls it makes have answered. When jobs arrive faster than the pool can run them, the backlog waits in the queue.
