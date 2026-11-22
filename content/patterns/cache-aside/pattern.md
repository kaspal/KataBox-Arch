The app asks the cache for a key before it goes anywhere else. If the key is there (a hit), the answer goes straight back and the database never hears about it. If it is missing (a miss), the app reads the database, stores the result in the cache with a time to live, and then answers.

Writes go to the database. The cached copy is left as it is, so readers may see the old value until it expires, unless the app also deletes the key when it writes. A short TTL limits how stale a read can be; a long one gives more hits.

The cache only fills with keys that someone has asked for, so its memory goes to the data that matters. The price is the miss path: after a restart or a flush, every read is a miss and the database takes the full load until the hot keys are back.
