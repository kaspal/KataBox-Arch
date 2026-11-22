## 1

What happens to the queue, and to everyone's wait, when submissions arrive faster than the judges can finish them?

## 2

Let the judge pool grow with the load instead of sizing it for a quiet afternoon. A deep queue keeps submissions safe; enough judges keep the wait short.

## 3

Put the Judge Workers inside an Autoscaling group whose maximum is several times the normal size, with a moderate target utilisation and a short scale delay. Keep an SQS Queue with a very large max depth in front of them, and a couple of Submit API servers behind the Load Balancer.
