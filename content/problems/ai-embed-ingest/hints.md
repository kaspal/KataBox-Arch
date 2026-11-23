## 1

Your account gets 600,000 tokens a minute, and a normal day uses about a third of that. What happens to a document that arrives when the minute's allowance is spent, and what happens to the person uploading it while the embeddings API takes its time?

## 2

A rate-limit error is a "not now", not a "never". Let the document wait somewhere durable and try it again a little later, instead of failing the upload. The workers that make the calls need enough slots to spend the whole allowance, but more slots than that only collect more 429s: the provider's limit is the real ceiling.

## 3

Have the App Servers (2 Go instances) publish each document to an Amazon SQS Queue with Maximum retries set to 10, so a refused call comes back after 1 s, 2 s, 4 s and so on. A Celery Worker with 2 instances and a concurrency of 12 consumes it, call the OpenAI API card (Model type Embeddings, 3,000 requests and 600,000 tokens a minute) and write the vector to a pgvector Vector Database.
