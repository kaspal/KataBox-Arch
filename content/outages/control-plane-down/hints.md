## 1

AZ-b's group kept asking for more instances, and none started. What did the design need in order to survive losing a zone, and was it available during the event?

## 2

Scaling out and replacing instances both go through the cloud's control plane, and it was down. A design that stays up through a zone loss must already be running enough capacity in the zones that are left, without starting anything new.

## 3

Add a third zone (AZ-c) with its own autoscaling group and App Server, and run six instances in each of the three zones, with a minimum of six. Two zones then carry twice the evening peak if one is lost.
