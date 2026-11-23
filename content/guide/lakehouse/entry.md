A lakehouse table is a set of data files, usually Parquet, in object storage, plus metadata that says which files make up the table right now. Formats such as Iceberg, Delta Lake and Hudi add what plain files lack: atomic commits, a schema, and a record of partitions and file statistics. Many engines, a warehouse, Spark jobs, stream processors, can then read and write one copy of the data at object-storage prices.

The table does no computing of its own; its **layout** decides what readers pay. With **partitioning** by day or hour, a query that filters by time reads only the matching partitions. Without it, every query reads everything. Streaming ingest writes small files often, and each file adds open and list overhead to every read, so **compaction** rewrites them into files near `target_file_mb`.

The simulator models that effect: a warehouse or batch job reading the table scans the bytes of the partitions it keeps and pays a cost per file, and the number of files depends on ingest and on the compaction schedule.

**How the simulator bills this.** The table is billed for its stored data only, at $0.023 per GB a month (S3 Standard): ten terabytes cost about $230 a month. The engines that read it are separate cards with their own prices.
