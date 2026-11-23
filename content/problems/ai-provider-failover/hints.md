## 1

With one provider, its bad minute is your bad minute. A second provider helps only if your calls actually move to it. A provider that is down is noticed in a few seconds, but one that accepts your call and never answers looks healthy. How long does your App Server wait for it, and what happens to the message that was waiting?

## 2

Stop waiting at some point: a call timeout a little above your longest normal answer. Once many calls to one provider fail, stop sending it calls for a while (a circuit breaker), so the failures cost nothing instead of a timeout each. Then try the failed call again: the next try goes to the other provider. And the other provider must be able to take the whole load on its own, within its limits.

## 3

Put an AWS ALB in front of 2 Go App Servers with a Call timeout of 5,000 ms, the Circuit breaker at errors at 20% and Breaker open for 60 s. Put the App Servers in an Availability zone with a Linkerd Service mesh and Mesh retries 3. Wire them to a Claude API card (4,000 requests and 2,000,000 tokens per minute) and an OpenAI API card (1,500 requests and 1,000,000 tokens per minute), and set the Web client's Timeout to 30,000 ms.
