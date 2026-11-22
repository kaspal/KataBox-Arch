## 1

Una consulta siempre va al mismo shard del índice de búsqueda. ¿Qué le pasa a ese shard cuando unas pocas consultas forman la mayor parte del tráfico?

## 2

Pon una caché de consultas pequeña delante del motor de búsqueda, para que las consultas populares se respondan desde memoria y el índice vea sobre todo la cola larga. Para las actualizaciones, deja que la captura de cambios de la base de datos alimente un stream y que unos Workers escriban cada cambio en el índice; luego comprueba cuántos cambios por segundo puede mover el stream.

## 3

Conecta una Cache Redis con un TTL corto, de unos 30 s, entre la Search API y Elasticsearch: 200 000 respuestas de 8 KB caben en la memoria por defecto. Conecta PostgreSQL a un stream Kafka para los cambios y deja que lo consuman un par de Indexers Celery. Un stream procesa un mensaje por partición a la vez, y cada cambio tarda unos 30 ms en indexarse, así que sube las particiones muy por encima de las 6 por defecto, hasta unas 24.
