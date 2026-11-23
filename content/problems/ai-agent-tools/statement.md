# Support Agent with Tools

## The product

Your online shop is adding a support agent. A customer asks something about an order ("where is my parcel?", "I want my money back"), and the help page hands the request to your backend as a task. A worker runs the task as an agent: it asks a hosted language model what to do, calls the tools the model asks for, gives the model the results, and repeats until the model writes the answer, which is then emailed to the customer. The tools are APIs: your own order system, the carrier's parcel tracking and the payment provider. Nobody watches the task run, but customers expect an answer within half a minute, and a task must never get lost.

## What you must meet

| Goal | Target |
| --- | --- |
| Peak traffic | 2 tasks/s |
| p95 completion latency | 25 s or less |
| Availability | 99% or more |
| Errors | 1% or less |
| Budget | $150,000 per month |
| Lost tasks | None |

## The traffic

Each task reaches your App Servers, possibly through an edge component, and they publish it to a broker. A Worker takes it from there and makes every call of the task itself, one after another:

- **Answer an order question** (80%): the model, then the order system (`orders`), then the carrier's tracking (`tracking`), then the model again for the answer.
- **Handle a refund** (20%): the model, the order system, the model, the carrier's tracking, the model, the payment provider (`payments`), and the model once more. Four model calls instead of two.

Every model call sends about 1,500 tokens (the conversation so far plus the tool results; 3,000 at p95) and gets about 100 back (250 at p95). The model is the Claude API flavour of Model Inference. Keep its Time to first token and speeds as they are: they describe the provider, not your design. Your account allows **4,000 requests** and **2,000,000 tokens a minute**: the simulator applies them to every Claude API card (the Inspector shows them, read-only).

The tools are reached over the network, with no card of their own:

| Tool | Median | p99 |
| --- | --- | --- |
| `orders` | 80 ms | 400 ms |
| `tracking` | 300 ms | 1,500 ms |
| `payments` | 200 ms | 800 ms |

The agent can't do without the order system or the payment provider: when one of those calls fails, the task fails. It can do without tracking: when the tracking call fails, or the Worker stops waiting for it, the agent goes on and answers from the order record. A Worker keeps its slot for the whole task, including every call it is waiting on.

Latency is measured to completion: from the moment the task arrives until the model's last answer comes back.

## The tests

One test is visible. You can run it as often as you like:

- **A normal day**: the traffic above, at peak, for 2 minutes.

Two more tests are hidden, and you see only their names: **The carrier's tracking API slows to 40 s a call** and **Returns week: five times the refunds**. A real returns week lasts days; the test runs that mix for 2 minutes. Your design has to pass all three.
