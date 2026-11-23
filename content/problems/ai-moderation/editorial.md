# Editorial

## Where the load goes

At 1,500 posts a second, every post goes through the classifier once. A typical post is about 40 tokens: Triton takes about 5 ms to start and reads the text at about 5,000 tokens a second, so a check takes about 5 + 8 = 13 ms (about 23 ms for a long post of 90 tokens). So 1,500 × 0.013 s keeps about 20 posts in flight. One GPU instance has 64 Batch slots, so on a normal day it is about a third busy.

The hidden tests change that. "A post goes viral" multiplies the traffic by 5 from 15 seconds in, for 30 seconds: 7,500 posts a second, about 98 in flight, more than one instance's 64 slots. "A model server is lost" kills the first Model Inference card 20 seconds in, and it does not come back. Every post sent to that card has nowhere else to go.

## The reference design

An AWS ALB sends the posts to 4 Go App Servers. They call two NVIDIA Triton Model Inference cards, each with 2 Instances, Model type set to Classifier and the default 64 Batch slots per GPU, and spread the calls over both. This costs about $3,241 per month: $1,468 for each Triton card, $280 for the App Servers and $25 for the ALB. The GPUs are most of the bill. A normal day has a p95 of about 36 ms with no errors, and each Triton card is about 8% busy.

The 2 instances on each card are not there for the normal day. Each card has 128 slots, enough to carry the whole viral spike (about 98 posts in flight) on its own. During "A post goes viral", the two cards share it, and the p95 stays at about 37 ms with no errors.

"A model server is lost" allows 3% errors and 97% availability. The design fails about 1.7% of the posts, all in the 2 seconds or so before the health checks notice the dead card: until then, half the calls still go to it. After that, every call goes to the other card, which carries the full load with about a third of its slots, and the p95 stays at about 36 ms.

For the cost challenge, a proof design comes in at 53% of the reference's cost, under the 55% target. It keeps two Triton cards, with one instance each, and 3 App Servers. Together the two single-instance cards still have 128 slots for the spike, and either one carries a normal day alone; what it gives up is the room to lose a card during a spike.

## Why the first try breaks

A common first try is the same edge and App Servers with a single NVIDIA Triton card at 1 instance. On a normal day it is about a third busy and looks oversized. It costs about $1,039 and passes "A normal day" with a p95 of about 36 ms.

When a post goes viral, about 98 posts want a slot at once and the instance has 64. The rest wait in the queue, the queue fills, and posts are dropped: about 32% errors over the run (about 40% of each second of the spike), with a p95 of about 519 ms. A second instance on that card would carry the spike.

When the model server is lost, there is no second one. Every post from 20 seconds in fails: about 67% errors over the run. More instances on the same card would not help, because the whole card is gone.

The point is this: a GPU tier that looks idle on an average day is sized for the average. Size it for the spike, and split it over at least two model servers so that losing one is a short blip.

## What the simulator leaves out

The classifier's accuracy is not simulated: a post counts as checked whether the label was right or wrong, and nothing here measures false positives or missed posts. There is no review queue for the posts the classifier flags, and no fallback that holds a post for later. An autoscaling group can't hold a Model Inference card in the simulator, so the model tier here has a fixed size; a real new GPU node also has to start and load the model before it serves, and that is not simulated. Images, video and the post's storage are not simulated either.

## Things to try

- Give the first try's Triton card 2 instances and run "A post goes viral". Then run "A model server is lost" again.
- Drop both Triton cards to 1 instance and run the spike. Watch how busy the slots get.
- Set the first try's Batch slots per GPU to 32 and run "A normal day". How close does it get to full?
