## 1

What is the person in the chat waiting for? Does their answer need the trace to be in the warehouse already? And if the warehouse has a slow minute, who should notice: the dashboards, or the person in the chat?

## 2

The dashboards may be minutes behind, so the trace doesn't have to reach the warehouse before the answer leaves. Hand it off: the App Server publishes the trace to a Stream, which keeps it safe, and a consumer writes it to the warehouse at its own pace. Then a slow warehouse only makes the dashboards a little late. Size the Stream's partitions and the consumer for the launch: each write holds a consumer slot for as long as the warehouse takes.

## 3

Put an AWS ALB in front of 4 Go App Servers, with the Web client's Timeout at 30,000 ms. The App Servers call a Claude API with the Requests per minute limit at 4,000 and the Tokens per minute limit at 2,000,000, and publish each trace to an Apache Kafka Stream with 12 partitions. An Apache Flink Stream Processor (2 instances, Concurrency per instance 8) consumes it and writes the traces to a Snowflake Data Warehouse with its default settings.
