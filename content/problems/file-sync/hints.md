## 1

Most uploads are chunks you already have. What does it cost you, in time and in money, to store such a chunk again, write its record again and tell every device about it again?

## 2

Give each chunk an identity and keep an index of the chunks you have. A quick lookup before the upload lets you skip the object store, the database write and the change notice for a repeat. S3 and SQS charge per request, so every chunk you skip is money saved, and an upload that never touches the object store does not feel it slow down.

## 3

Put a Redis Cache next to the Sync API as a chunk index, used cache-aside, with a long TTL such as a day (86,400 s): 1 GB is plenty for an entry per chunk. Keep S3 for the chunks, PostgreSQL for the file metadata with a replica to share the folder listings, and an SQS Queue with a few Celery Notifiers behind it.
