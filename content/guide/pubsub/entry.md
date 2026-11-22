Publish/subscribe, or **pub/sub**, delivers one message to many receivers. A publisher sends a message to a **topic**, and the topic pushes a copy to every **subscriber**. The publisher doesn't know who is listening, so new subscribers can join without any change on its side.

It is the usual way to join servers that each hold some of the users' live connections. A chat message arrives at one server, which publishes it; every server subscribed to the conversation receives it and passes it on to its own users.

Pub/sub differs from a queue in two ways. Every subscriber gets every message, so the work grows with the number of subscribers. And many topics keep little or nothing, so a subscriber that is slow or offline either slows everyone down or misses messages. When a subscriber needs a buffer, put a queue between it and the topic.
