## 1

The fraud model is a call in the middle of checkout. What happens to a payment, and to the API that holds it, if that call suddenly takes several times longer than usual?

## 2

An in-line model call needs a latency budget. The score is optional: when it is late, the rules can decide, and the payment goes on. So stop waiting for the model after a set time, well under the 60 ms p95, and leave room for the cache read. Without a limit, a slow model makes every payment slow, and the waiting requests fill the API until it fails.

## 3

Partner servers → an AWS ALB → 6 Go API instances with a Call timeout of 40 ms. The API reads features from a Redis cache (Memory 16 GB) and calls an NVIDIA Triton Model Inference card with 2 instances and Model type set to Classifier. For events, the API publishes to an Apache Kafka stream with 24 Partitions, consumed by Apache Flink with 2 instances, which writes the features to the same Redis.
