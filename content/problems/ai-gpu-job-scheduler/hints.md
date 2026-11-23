## 1

On a normal day the cluster keeps up, so the order of the queue hardly matters. On release day urgent jobs ask for more GPU time than the cluster has. Who should wait: the researcher blocking a release, or a sweep nobody is watching? And how many GPUs does the urgent work alone need when a node is gone?

## 2

A first-in, first-out queue makes every urgent job wait behind all the bulk jobs ahead of it, and when the work outruns the GPUs that pile only grows. Let the queue hand out urgent jobs first, so bulk jobs take only the GPUs left over. Then size the cluster for the urgent load at its peak with one node lost, not for the average load.

## 3

Keep the Go App Servers (2 instances) and a RabbitMQ Queue with "Live before bulk (priority)" on and a visibility timeout of 60 s. Wire it with Consume to 5 GPU nodes, each a Celery Worker with 8 Instances, "Concurrency per instance" 1 and a service time of 2,000 ms.
