A dead-letter queue, or **DLQ**, catches the messages that a queue or stream could not deliver. When a message has failed every retry, the broker moves it to the DLQ instead of dropping it. Nothing there is processed automatically: it waits for a person or a tool to look at it, fix the cause and replay it.

It protects against two problems. A provider outage that outlasts the retries would otherwise throw away every message sent during it. And a **poison message**, one that can never succeed, would otherwise be retried for ever and waste the workers' time.

A DLQ only helps if someone watches it. Alert when messages land there, and keep a simple way to send them back to the main queue once the cause is fixed. The retention setting is how long they wait before they expire.

**How the simulator bills this.** A dead-letter queue is billed like an SQS queue: $0.40 per million requests, three per message (send, receive and delete), so $1.20 per million messages that land in it. There is no fee per queue, so a DLQ that stays empty costs nothing (list price, us-east-1, read 2026-11-22).
