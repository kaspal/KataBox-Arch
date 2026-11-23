# Image Thumbnails

## The product

You run the photo backend for a group of mobile apps. People take a picture, tap upload, and expect to see it in their gallery as a small preview a moment later. Each upload is a full photo of about 2 MB. Storing the original is quick. Making the thumbnails is not: a server has to decode the whole image and resize it, and that takes real time. The upload should not sit and wait for that work, but the work must always get done. A photo that never gets its thumbnails is a broken photo.

## What you must meet

| Goal | Target |
| --- | --- |
| Peak traffic | 300 uploads/s |
| p95 completion latency | 5 s or less |
| Availability | 99.9% or more |
| Budget | $14,500 per month |
| Lost messages | None |

## The traffic

There is one request type.

- **Upload a photo** (100%). The upload may pass an edge component such as a Load Balancer, then reaches the Upload API, an App Server. It writes the 2 MB original to object storage and publishes a thumbnail job to a broker. A Worker takes the job, decodes and resizes the photo, and writes the thumbnails back to object storage.

Latency is measured to completion: from the moment the upload arrives until the thumbnails are stored, not just until the Upload API answers. Decoding and resizing a 2 MB photo takes a Worker about 200 ms, and it keeps its slot for the whole job.

## The tests

One test is visible. You can run it as often as you like:

- **Normal day**: the traffic above, at peak.

Two more tests are hidden, and you see only their names: **Upload burst** and **A worker crashes**. Your design has to pass all three.
