# Solución comentada

## Adónde va la carga

Con 10 000 peticiones/s y un 97 % de lecturas, unas 9 700 peticiones/s abren un feed y unas 300 publican un post. Cada lectura devuelve una página de 16 KB. Los posts parecen poca cosa, pero cada uno se multiplica: con 10 seguidores de media, 300 posts se convierten en unas 3 000 escrituras de feed por segundo, y un solo post de una cuenta con 50 veces más seguidores se convierte en unas 500. Si esas escrituras y las lecturas comparten una base de datos, se pelean por las mismas conexiones.

## El diseño de referencia

Un Load Balancer reparte el tráfico entre 6 App Servers Go. Una lectura consulta primero una Cache Redis de feeds de 16 GB. Un acierto termina la petición; un fallo lee de PostgreSQL, que funciona con una principal y una réplica, y llena la caché. Un post se escribe en la principal y se publica en un Stream Kafka con 32 particiones, y el App Server queda libre. 4 Workers de fan-out (Celery) consumen el Stream y escriben el post en el feed de cada seguidor dentro de la Cache.

Cuesta unos 1 866 $ al mes y su p95 ronda los 23 ms en un día normal. Supera las dos pruebas ocultas. Los feeds ya están hechos cuando llegan los lectores, y una ráfaga de posts solo mantiene a los Workers más ocupados durante un rato, sin frenar el camino de lectura.

## Por qué falla el primer intento

Un primer intento habitual mantiene los mismos App Servers, publica los posts en una Queue RabbitMQ y hace que los Workers escriban filas de feed en la única base de datos PostgreSQL. Las lecturas van directas a esa base de datos. Pero cada lectura de 16 KB y cada escritura de feed caen en la misma base de datos, y el p95 ronda los 155 ms en un día normal.

Las pruebas ocultas aprietan más. «Todos abren la app a la vez» triplica el tráfico durante 15 segundos. La base de datos no tiene margen, los hilos se acumulan y falla alrededor del 33 % de las peticiones. «Noticia de última hora» envía 4 veces más posts. Las escrituras de feed ocupan más parte de la base de datos, las lecturas esperan detrás y falla alrededor del 5 % de las peticiones.

## Para probar

- Quita la Cache del camino de lectura y vuelve a ejecutar «Un día normal». Observa el p95 y la carga de la base de datos.
- Reduce la Cache de feeds a 2 GB y compara la tasa de aciertos.
- Quita Workers de fan-out de uno en uno y busca cuántos bastan para seguir el ritmo.
