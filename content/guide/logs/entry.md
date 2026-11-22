Logs are the lines each service writes as it works: a request arrived, a query failed, a payment was refused. A log stack ships those lines from every server to one place, stores them and lets you search them, so you don't have to log in to each machine to find out what happened.

Where metrics say *that* something is wrong, logs say *what* went wrong, with the full detail of each event. They are most useful when they are **structured**, with named fields rather than free text, and when every line carries a request id that ties it to the rest of the request's story.

Logs never sit on a request's path, so in the simulator they add no latency and cannot drop a request. They only add their monthly cost, and a problem may require them as part of a production-ready design.
