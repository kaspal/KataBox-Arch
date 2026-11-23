A batch compute cluster takes a large job, such as rebuilding a report from a day of logs, and splits the work into many **tasks**. Each node runs several **executors**, and each executor runs one task at a time. More nodes means more tasks in parallel, so the job finishes sooner, as long as the work is spread evenly.

It rarely is. A job is done only when its **last** task is done. If one key holds much more data than the others (**skew**), its task is huge and every other executor waits for it. **Salting** splits the biggest tasks into pieces so several executors share them. **Speculation** starts a second copy of a slow task on a free executor near the end; that helps when a task is slow by bad luck, but not when it is slow because of skew, since the copy has just as much work.

Clusters often run part of their nodes on cheaper, **interruptible** capacity (spot). When such a node is taken back, its tasks start again from zero, unless the job writes **checkpoints**, in which case they resume from the last one. The problem's deadline for the job is what you are graded on.

**How the simulator bills this.** Every flavour bills $140 a month per node, an m5.xlarge-class machine ($0.192 an hour for 730 hours). The simulator doesn't apply a spot discount yet, so spot nodes cost the same as regular ones for now. These are teaching prices; source to be confirmed.
