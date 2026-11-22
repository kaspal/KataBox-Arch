Instead of writing to the database and then publishing to a broker, the service writes both the change and an outbox row describing the event in one local transaction. Either both are saved or neither is, so the two can never disagree.

A separate relay reads new outbox rows and publishes them to the stream. It can poll the table, or follow the database's change log (change data capture). Once the broker accepts an event, the relay marks the row as sent.

If the relay crashes after publishing but before marking the row, it publishes the event again on restart. That is why consumers must handle duplicates, usually by remembering the ids they have already processed.
