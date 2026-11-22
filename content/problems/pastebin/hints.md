## 1

What does it cost a relational database to store a 1 MB value, and what is it unable to do while it is busy with one?

## 2

Keep paste bodies in object storage, which takes about the same time whatever the size, and cache the popular reads so they skip the store entirely.

## 3

Wire the App Servers to Object Storage (S3) for paste bodies and to a Redis Cache of a few GB for hot pastes; you do not need a SQL Database at all. Each create keeps an App Server thread busy while it uploads, so run about 4 Go App Servers to leave room for a burst.
