## 1

The site has 20,000 files and they rarely change. Does a server really need to fetch the same file every time someone opens a page?

## 2

Cache the files at the edge, close to the visitors. When most requests are answered there, the origin leaves the hot path, and its troubles stop reaching your visitors.

## 3

Put a CDN with a long TTL in front of Object Storage that holds the site files, and wire the Client only to the CDN. Let the CDN forward the contact form to a Load Balancer and one small App Server that writes to the database.
