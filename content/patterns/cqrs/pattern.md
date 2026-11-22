Commands, the requests that change things, go to a write model that checks the rules and stores the result in a normalised database. Each change is also published as an event.

Projectors consume those events and keep one or more read models up to date: a document per screen, a search index, a ranked list. Queries go only to the read models, which are shaped so that each answer is a single cheap lookup.

The read side is always a little behind, so a user may not see their own change at once. Show the result from the command's reply, or read from the write side for that user, where it matters. To publish the event reliably alongside the write, pair CQRS with a transactional outbox.
