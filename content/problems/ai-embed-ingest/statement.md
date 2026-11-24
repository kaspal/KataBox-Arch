# Embedding Ingest

## The product

You run the search behind a help-centre product. Your customers add their own documents (articles, manuals, policies), and their users then search them by meaning, not only by exact words. To make a document searchable, your backend sends its text to a hosted embeddings API, gets a vector back, and stores it in a vector database. You don't run the model yourself: you pay for it per token, and your account may send only so many requests and tokens a minute. Nobody waits at a screen for one document, but a document should show up in search soon after it is added, and it must never silently go missing.

## What you must meet

| Goal | Target |
| --- | --- |
| Peak traffic | 5 documents/s |
| p95 completion latency | 30 s or less |
| Availability | 99.9% or more |
| Errors | 0.1% or less |
| Budget | $28,000 per month |
| Lost messages | None |
| Dead-lettered messages | None |

## The traffic

There is one request type.

- **Add a document** (100%). The upload reaches your App Servers, possibly through an edge component. Then the document's text goes to a Model Inference card with Model type set to Embeddings, and the vector it returns is written to a Vector Database. The App Server can make these calls itself, or publish the document to a broker for a Worker to make them.

A document that ends up in a Dead-Letter Queue is kept, but it never reaches search, so the judge counts it: none is allowed.

Latency is measured to completion: from the moment the upload arrives until the vector is stored. A typical document is about 500 tokens long; one in twenty is longer than 1,500 tokens. Because of those long ones, the average is about 625 tokens.

The embeddings API is the OpenAI API flavour. Keep its Time to first token (890 ms) and Prompt speed (5,000 tokens a second) as they are: they describe the provider, not your design. Your account's limits are **3,000 requests a minute** and **600,000 tokens a minute**. They are not a design choice: the simulator applies them to every OpenAI API card (the Inspector shows them, read-only). A call over a limit is refused with a rate-limit error (an HTTP 429). Use the Real time API mode: the batch API returns its results hours later, far past the 30 s goal.

## The tests

One test is visible. You can run it as often as you like:

- **A normal day**: the traffic above, at peak.

Two more tests are hidden, and you see only their names: **A customer imports their archive (×10 for 45 s)** and **The embeddings API slows down**. Your design has to pass all three.
