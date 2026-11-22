A queue sits between the part of a system that asks for work and the part that does it. A producer, usually an app server, **publishes** a message and moves on. A **consumer**, usually a worker, takes the next message when it has a free slot, does the work and acknowledges it. Each message goes to one consumer only.

The queue turns a burst into a backlog. When traffic spikes, messages wait instead of failing, and the workers catch up afterwards. The price is time: the work finishes later, so the user gets a quick "accepted" and the real result arrives on its own schedule.

Queues also make failure survivable. A message whose work fails is tried again after a short wait, and the wait grows with each try. If it keeps failing, it moves to a **dead-letter queue** for a human or a tool to look at, instead of vanishing.

**How the simulator bills this.** Amazon SQS bills $0.40 per million requests, and each message takes three, a send, a receive and a delete: $1.20 per million messages. Each retry adds one more receive. There is no fee per queue, and batching isn't modelled. RabbitMQ is self-hosted on one EC2 m5.large node at $70 a month, with no fee per message; managed Amazon MQ would be about $210 (list price, us-east-1, read 2026-11-22).
