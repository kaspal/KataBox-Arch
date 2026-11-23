## 1

Cart views never call the recommendations service, yet they failed too. What were they waiting for on the App Server?

## 2

Every product page held an App Server slot for as long as the recommendations call took, and with no call timeout that was 3 s. 30% of the traffic waiting 3 s each needs far more than 256 slots. The page can render without recommendations, so the app does not have to wait.

## 3

On the App Server, set a call timeout of 200 ms (the recommendations step is optional, so a timed-out call goes on without them) and turn on the bulkhead, so the recommendations calls get their own share of the slots. Nothing else needs to change.
