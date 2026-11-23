## 1

The App Servers and the database were nearly idle during the outage. Which card refused the requests, and why?

## 2

The limiter couldn't reach Redis, so it couldn't tell whether a request was within its key's limit, and it was set to refuse anything it couldn't count. For a general API limit, refusing everyone is worse than limiting no one for a while.

## 3

On the Rate Limiter, set On failure to open. Nothing else needs to change: the App Servers have room for the unlimited traffic while Redis is down.
