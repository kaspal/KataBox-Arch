## 1

Durante un incidente todos los servicios escriben el triple de logs, y un consumidor puede caerse en cualquier momento. ¿Qué limita cuántos lotes archiva tu pipeline a la vez, y qué pasa con los lotes que tenía un consumidor caído?

## 2

Un stream entrega un mensaje por partición cada vez, así que el número de particiones limita cuántos lotes se archivan a la vez; dale particiones suficientes para un flujo triple y huecos de Worker suficientes para mantenerlas todas ocupadas. Después reparte esos Workers en más de una tarjeta. Cuando cae un consumidor, los lotes que tenía esperan a que venza el tiempo de sesión del stream antes de que los demás se hagan cargo, y si era tu único consumidor, ya no se envía nada.

## 3

Usa un stream Kafka con unas 128 particiones y dos tarjetas de Shippers Celery con 10 instancias cada una: con la concurrencia por defecto de Celery, 8, son 160 huecos, y cualquiera de las dos tarjetas puede con un día normal ella sola. Los Shippers escriben en S3; la Collector envía las búsquedas a Elasticsearch.
