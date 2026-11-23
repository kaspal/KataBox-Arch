## 1

On a normal day, one GPU looks mostly idle. How many posts does the classifier work on at once at the normal peak, and how many when a viral post brings five times the traffic? And if that GPU server stops, what checks the next post?

## 2

Posts in flight = posts a second × time per post. At 1,500 posts a second and about 13 ms each, that is about 20 at once; five times the traffic is about 98, more than the 64 Batch slots of one GPU instance. Size the model tier for the spike, not the average. And put the classifier on two separate model servers, each able to carry the whole load, so losing one costs a moment, not the product.

## 3

Put an AWS ALB in front of 4 Go App Servers. Wire them to two NVIDIA Triton Model Inference cards, each with 2 Instances and Model type set to Classifier. The App Servers spread the calls over both, and either one alone can carry the viral spike.
