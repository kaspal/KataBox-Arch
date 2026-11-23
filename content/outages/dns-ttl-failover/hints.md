## 1

Route 53 noticed that us-east was gone within seconds and started answering with us-west. So why did us-west never get a request? Look at the DNS card's props.

## 2

Clients and resolvers keep a DNS answer for its TTL, and only ask again when it expires. With a TTL of 86,400 s, they keep going to us-east for a day. And once they do move: can one App Server in us-west carry 6,000 requests a second (9,000 at a busier hour)?

## 3

On Route 53, set the TTL to 30 s and keep routing on failover. On the us-west App Server, set instances to 3 (one Go server here carries a bit under 5,000 requests a second). Nothing else needs to change.
