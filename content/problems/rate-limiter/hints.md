## 1

When one customer misbehaves, who should pay for it: that customer, or everyone sharing your App Servers?

## 2

Give every API key its own token bucket at the edge, so one key running out of tokens never touches anyone else's.

## 3

Put a Rate Limiter in front of the App Servers instead of a plain Load Balancer. Set the per-key limit well above a real customer's busiest moment, at least twice the normal 59 requests/s, and add a burst on top.
