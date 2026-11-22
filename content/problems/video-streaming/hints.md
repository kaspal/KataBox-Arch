## 1

How many times is the same popular segment fetched in a minute, and does your origin really need to serve it every time?

## 2

Put the segments behind a CDN so popular ones are answered at the edge, and keep transcoding off the request path: the upload only needs to be stored and queued before the creator gets an answer.

## 3

Place a CDN with a long TTL (a day is fine, segments never change) in front of Object Storage that holds the segments. Let the CDN forward uploads to a Load Balancer and a couple of Upload API servers, which write to Object Storage and publish to a Queue drained by several Transcoder Workers.
