## 1

The spike lasted ten seconds, but the errors didn't stop when it did. What was the Profile service still busy with after 25 s, and who was waiting for those answers?

## 2

Profile's queue could hold about 250 ms of work, longer than the mesh's 200 ms timeout. Calls timed out while they were still queued, the service did the work anyway, and the mesh sent each one again, three times. Work that waits longer than its caller's timeout is done for nobody.

## 3

On the Profile service, set Shed load past queue to 8. Keep the timeouts and retries as they are: the excess is now turned away at once instead of piling up.
