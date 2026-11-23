## 1

A completion on an idle GPU takes about 50 ms, so a budget of 150 ms looks generous. What happens to that time when the GPUs are busy, when the typing quadruples, and when one card is gone? And does every keystroke really need a GPU?

## 2

Two things keep you inside the budget. First, headroom: a fuller batch writes every completion slower, and a full one makes requests wait, so plan GPUs to stay well short of full at four times the traffic, spread over cards of the same size so the loss of one leaves enough. Second, a cache in front of the model: an identical prefix gets the same completion, and every request the cache answers is one the GPUs never see, at four times the traffic too. Compare the price of a cache with the price of the GPUs it saves.

## 3

Put an AWS ALB in front of 6 Go App Servers. They read a Redis Cache with 4 GB of Memory and a TTL of 3,600 s, and on a miss call three vLLM on GPU cards, each with 2 Instances, Batch slots per GPU at 64 and the model's speeds from the brief. On the Web client, set the Timeout to 2,000 ms, Retries to 2, the Retry backoff to 20 ms and the Backoff jitter to Full (random wait).
