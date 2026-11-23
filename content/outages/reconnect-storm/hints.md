## 1

Only one gateway died, yet almost every connection attempt failed for the rest of the match. When did the dead gateway's clients try to reconnect, and where could they go?

## 2

Two things went wrong together. With no reconnect jitter, all 100,000 clients arrived in the same instant and, once refused, came back together every second. And the survivors were 91% full: even spread out, there was room for only 30,000 of them. Fixing just one is not enough: in the simulator, jitter alone still has most attempts refused, and headroom alone still costs about fifteen seconds of storm.

## 3

On the Client, set a reconnect jitter of 10 s. On each WebSocket Gateway, raise the connection limit to 135,000, so three gateways can hold all 400,000 fans. Nothing else needs to change.
