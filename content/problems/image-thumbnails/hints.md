## 1

Resizing a photo takes a fifth of a second. What should the upload do while that happens, and what happens to the job if the machine doing it disappears halfway through?

## 2

Put the thumbnail jobs on a queue and let Workers take them at their own pace. Count how many Worker slots the peak keeps busy, then make sure that losing one Worker card still leaves enough to keep up.

## 3

Publish each job to an SQS Queue and wire two separate Celery Worker cards to it, each big enough to carry the whole peak by itself: about 12 instances at the default concurrency of 8. If one card dies, the other keeps going, and the queue's visibility timeout hands the dead card's unfinished jobs back to it.
