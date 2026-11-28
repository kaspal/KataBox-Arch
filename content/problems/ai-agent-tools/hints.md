## 1

While a tool takes its time, what is the Worker doing with its slot? And if nothing ever stops waiting, how long can one task take?

## 2

A Worker card keeps about (tasks a second) × (seconds per task) slots busy at once. If tracking takes 40 s a call, every task holds its slot 40 s longer, and the slots run out within seconds. A Call timeout on the Worker caps how long it waits for any one call, and because tracking is optional, the agent then goes on without it. The model calls must still fit under that timeout. Then count the slots again for a day with many refunds: each refund makes twice the model calls.

## 3

A Web client sends tasks to a Go App Server, which publishes them to an Amazon SQS Queue with Maximum retries at 6. A Celery Worker with 5 Instances and the Call timeout set to 8,000 ms consumes them and calls a Claude API Model Inference card with the Tokens per minute limit at 2,000,000.
