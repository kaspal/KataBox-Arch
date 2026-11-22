The app publishes one event to a topic and moves on. The topic keeps a list of subscribers and hands each of them its own copy. One subscriber might update feeds, another send an email, another refresh a search index, each at its own pace.

The publisher does not know who is listening, which keeps it small and stable. The cost moves downstream: one event can turn into a large amount of work, and a slow subscriber falls behind without the others noticing.

Fan-out on write precomputes results, such as a feed per follower, so reads are cheap. For accounts with huge audiences that write is too expensive, and the usual answer is a hybrid: fan out for most authors and merge the biggest ones in at read time.
