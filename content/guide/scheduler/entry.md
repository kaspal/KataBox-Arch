A scheduler starts work on a timetable. Instead of waiting for a user to ask, it fires jobs every minute, every night or on whatever schedule you set: reports, clean-ups, crawls, syncs. In a design it plays the part a client plays for user traffic: it is where the work comes from.

Nobody waits for the answer, so a scheduled job is judged by when it finishes, not by how fast something replies. That makes it a natural partner for a queue. The scheduler publishes the jobs, the queue holds them, and workers drain it at their own pace, even when many jobs fall due at the same moment.

Simple schedulers such as cron run commands on one machine and forget about them. Workflow tools such as Airflow add dependencies between steps, retries and a record of every run, at the cost of more to set up and operate.
