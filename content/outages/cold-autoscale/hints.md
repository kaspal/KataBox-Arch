## 1

The autoscaler noticed the wave within seconds. Look at when the new instances actually started serving, and at when the wave ended.

## 2

A scale-out that takes 60 s can't help with a wave that rises in ten seconds and lasts a minute. The capacity has to be running before the wave, or arrive in seconds: more instances kept running, a lower target so there is room to absorb the start, and a much shorter delay.

## 3

On the autoscaling group, set the minimum to 18 (and the App Server to 18 instances), the target utilisation to 40% and the scale delay to 5 s. Nothing else needs to change.
