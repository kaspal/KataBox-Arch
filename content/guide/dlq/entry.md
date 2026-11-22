A dead-letter queue, or **DLQ**, catches the messages that a queue or stream could not deliver. When a message has failed every retry, the broker moves it to the DLQ instead of dropping it. Nothing there is processed automatically: it waits for a person or a tool to look at it, fix the cause and replay it.

It protects against two problems. A provider outage that outlasts the retries would otherwise throw away every message sent during it. And a **poison message**, one that can never succeed, would otherwise be retried for ever and waste the workers' time.

A DLQ only helps if someone watches it. Alert when messages land there, and keep a simple way to send them back to the main queue once the cause is fixed. The retention setting is how long they wait before they expire.
