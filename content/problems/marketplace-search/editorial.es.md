# Solución comentada

## Adónde va la carga

Con 5000 peticiones/s, unas 4750 son búsquedas y unas 250 son actualizaciones de anuncios. Cada búsqueda devuelve unos 8 KB, unos 38 MB/s de resultados. Las 200 000 respuestas juntas ocupan unos 1,6 GB, así que caben en una Cache Redis de los 2 GB por defecto.

Elasticsearch tarda unos 20 ms por llamada, y una consulta siempre cae en el mismo shard. Con Zipf 1,1, las consultas más populares se amontonan en unos pocos shards, y el límite lo pone el más cargado, no la media.

Cada actualización termina con un viaje por el stream: un Indexer le dedica unos 10 ms y luego la escribe en Elasticsearch, unos 30 ms en total. Un stream mantiene un solo mensaje en curso por partición, así que 6 particiones mueven unos 6 ÷ 0,03 s = 200 cambios por segundo, menos que los 250 que llegan.

## El diseño de referencia

Un Load Balancer (AWS ALB) envía las peticiones a 8 instancias Go de la Search API, 512 huecos con la concurrencia por defecto de 64. Las búsquedas consultan primero una caché de consultas Redis con un TTL de 30 s y, si fallan, preguntan a Elasticsearch, con 4 shards. Las actualizaciones se escriben en PostgreSQL, conectado a un stream Kafka de 24 particiones por su arista de captura de cambios. 2 Indexers Celery, con un tiempo de servicio de 10 ms, consumen el stream y escriben cada cambio en Elasticsearch. Cuesta unos 1764 $ al mes, con un p95 de unos 50 ms.

«Una reindexación en hora punta» añade 30 ms a cada llamada a Elasticsearch a partir del segundo 15, durante 30 segundos, y admite un p95 de 400 ms. Solo se ralentizan los fallos de caché: el p95 sube a unos 143 ms. Cada cambio tarda ahora unos 60 ms; 24 particiones siguen permitiendo unos 400 por segundo y los 16 huecos de los Indexers (2 × la concurrencia por defecto de 8) unos 260, así que el índice sigue el ritmo. «Todo el mundo busca lo mismo» hace la popularidad de las consultas mucho más empinada (Zipf 1,5). La caché absorbe las consultas más calientes y el p95 baja a unos 38 ms.

Durante el ajuste, un stream con las 6 particiones por defecto de Kafka se quedaba cada vez más atrás; con 24, el índice se mantiene al día.

## Por qué falla el primer intento

Un primer intento habitual es el mismo diseño sin la caché de consultas, por unos 1728 $ al mes. Cada búsqueda llega a Elasticsearch, e incluso en un día normal el shard más cargado no da abasto, porque todas las consultas populares caen en él. Los App Servers se llenan esperándolo y se descarta cerca del 61 % de las peticiones, con un p95 de unos 4,5 s. La reindexación lo empeora, hasta cerca del 73 %, y las consultas calientes también, hasta cerca del 66 %.

Más shards no lo arreglan. Una consulta sigue yendo a un solo shard, así que el caliente sigue caliente: con 10 shards se seguían descartando peticiones durante el ajuste. Una caché pequeña delante de las consultas calientes hace lo que más shards no pueden.

## Lo que el simulador deja fuera

No se simulan la relevancia ni el orden de los resultados: cada respuesta es solo un resultado de 8 KB. Una reindexación real reconstruye el índice mientras sirve tráfico, y eso gasta disco, memoria y CPU; aquí son solo llamadas más lentas. Las réplicas del propio índice de búsqueda tampoco se modelan en detalle.

## Para probar

- Vuelve a dejar el stream en 6 particiones y observa cómo crece la latencia hasta completar de las actualizaciones a lo largo de la ejecución.
- Quita la caché de consultas y sube Elasticsearch a 10 shards. Compara el coste y los descartes.
- Baja el TTL de la caché a 5 s y vuelve a ejecutar «Un día normal».
