## 1

Count the tokens a minute that reach the large model's key on a normal day, and again when the easy questions triple. Which questions actually need the large model?

## 2

Easy questions are 80% of the traffic and more than half of the large model's tokens. Send them to the small model on your own GPUs: they stop counting against the key's 1,500,000 tokens a minute, and a GPU costs the same whether it serves 8 questions a second or 24. Then size those GPUs for the newsletter: an easy answer holds a batch slot for about a second and a half, and a GPU whose slots fill up slows every answer down.

## 3

Put a Kong API Gateway in front of 2 Go App Servers and 2 Microservices. Wire the App Servers to a vLLM on GPU card with 4 Instances and the Output speed at 150, and the Microservices to a Claude API card with the Tokens per minute limit at 1,500,000. On the Web client, set the Timeout to 60,000 ms.
