# Static Site + CDN

## The product

A company runs a marketing site. It is made of pages, images and scripts that change only when the team deploys a new version. There is one dynamic part: a contact form that visitors fill in, which must be saved to a database.

Almost every visitor only reads. They open the home page, scroll, look at a few product pages and leave. A small number of them send the form.

## What you must meet

| Requirement | Target |
| --- | --- |
| Peak traffic | 1,500 requests/s |
| Static files | 99% of requests |
| p95 latency | 60 ms or less |
| Availability | 99.9% or more |
| Budget | $4,000/month or less |

## The traffic

- **Page, image or script** (99%). The site has 20,000 files. A few of them, like the home page and the main script, are requested far more often than the rest (a Zipf 1.0 popularity). These responses are cacheable. A CDN can serve them, or an origin can: an App Server or Object Storage.
- **Contact form** (1%). An App Server receives the form and writes it to the database.

## The tests

Your design is judged by one visible test and two hidden tests.

- **Normal day** (visible). A steady day at peak traffic.
- **A deploy purges the CDN** (hidden).
- **The origin slows down** (hidden).

Every test must keep errors under 0.1%.
