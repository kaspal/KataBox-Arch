## 1

Nothing failed, and the burst lasted only 30 s, yet orders were still more than a minute late three minutes later. How fast can the workers work off what piled up in the queue?

## 2

The workers process 160 orders a second; the burst brought 500. Everything over 160 waited in a queue that would hold ten million, and each new order waited behind all of it. A queue helps only if it drains: limit how much it holds, so the excess is refused at once, and give it more workers when it fills.

## 3

On the queue, set max depth to 3,000. Put the workers in an Autoscale group (Kubernetes HPA) with 2 to 6 instances, a 50% target and a scale delay of 5 s.
