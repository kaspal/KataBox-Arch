## 1

A queue lets players wait instead of failing, but it only helps if the matchers can catch up. How busy are your matchers at peak, and what happens to the queue when twice as many players arrive?

## 2

Players want a match near them, so matchmaking splits naturally by Region. A full stack in each Region, with its own queue and its own matchers, halves the load each one sees, and if one Region is lost, DNS can send its players to the other.

## 3

Put Route 53 DNS (TTL 30 s) in front of two Regions, us-east and eu-west. In each, wire an AWS ALB to Go App Servers with 2 instances, a RabbitMQ queue, Celery matchers with 4 instances, a concurrency of 64 and a service time of 200 ms, and a DynamoDB table. Each Region matches its own players.
