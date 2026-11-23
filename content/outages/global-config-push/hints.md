## 1

The site ran in three Regions so that one bad Region couldn't take it down. Why did all three fail in the same second?

## 2

The Config card sends every change to every Region at once, and nothing reverts it until a person does. A config change should reach one Region first, wait, stop as soon as that Region's errors rise, and be revertible in a second.

## 3

On the Config card, turn on staged delivery (3 stages, a bake time of 30 s) and the kill switch. Nothing else needs to change.
