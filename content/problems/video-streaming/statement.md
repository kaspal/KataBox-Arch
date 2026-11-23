# Video Streaming

## The product

You run a video service. Viewers press play and the player fetches the video in short segments, a few seconds each, one after another. If a segment arrives late, the picture freezes and the viewer notices at once. A handful of shows are watched far more than the rest. On the other side, creators upload new videos, which must be turned into several renditions (sizes and bitrates) before anyone can watch them. Uploads are rare next to plays, but each one is heavy work.

## What you must meet

| Goal | Target |
| --- | --- |
| Peak traffic | 4,000 requests/s |
| Segment plays | 99% of requests |
| p95 latency | 80 ms or less |
| Availability | 99.9% or more |
| Budget | $17,000 per month |

## The traffic

- **Play a video segment** (99%). The request may pass an edge component such as a CDN, then is served by an origin: an App Server or Object Storage. There are 500,000 segments, their popularity follows a Zipf 1.1 curve, and every segment can be cached.
- **Upload a video** (1%). The request may pass an edge component, then reaches the Upload API on an App Server. The Upload API stores the video, then publishes it to a Queue. Transcoder Workers take it from there, spend about 400 ms on it, and write the renditions.

## The tests

One test is visible. You can run it as often as you like:

- **Normal evening**: the traffic above, at peak.

Two more tests are hidden, and you see only their names: **A season premiere** and **Everyone uploads at once**. Your design has to pass all three.
