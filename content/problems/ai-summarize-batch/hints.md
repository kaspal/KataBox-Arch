## 1

Count the tokens, not the requests. How many tokens does the whole backlog need, and how many does your account allow in a minute? Then ask the same question for a backlog three times bigger. Nobody waits for a single summary: does this work need the real-time API at all?

## 2

The Model Inference card has an "API mode". In real time, every call counts against the per-minute limits, and adding Workers only adds refused calls. In batch mode the token price is half, and the per-minute limits don't apply, so the only cap left is how many calls your own Workers can hold at once: documents a second × seconds per call.

## 3

Scheduler (Airflow) → Publish → Amazon SQS Queue (Maximum retries 10) → Consume → Celery Worker, 2 instances with a concurrency of 64. Wire the Queue to a Dead-Letter Queue with Publish. The Worker calls a Claude API Model Inference card over HTTP, with "API mode" set to "Batch (cheaper, slower)".
