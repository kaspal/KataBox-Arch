# Solución comentada

## Adónde va la carga

Con 1000 peticiones/s, unas 990 son lotes de log y unas 10 son búsquedas. A 256 KB por lote, son unos 250 MB de logs por segundo, unos 22 TB al día. Un Shipper dedica 5 ms a cada lote y lo escribe en S3 (unos 50 ms de mediana). Con los saltos y la cola lenta de las escrituras, archivar un lote lleva unos 65 ms.

Un stream entrega un mensaje por partición cada vez, así que las particiones limitan cuántos lotes hay en curso. A unos 65 ms por lote, 128 particiones archivan unos 2000 lotes por segundo y 96 unos 1500. Un día normal cabe en ambos casos. Un incidente que triplica los logs, unos 2970 por segundo, no cabe en ninguno, así que el stream guarda un atasco hasta que se vacía.

## El diseño de referencia

La Collector, con 2 instancias Go detrás de un Load Balancer, publica cada lote en un stream Kafka con 128 particiones. Lo consumen dos tarjetas de Shippers Celery, Shippers A y Shippers B, cada una con 10 instancias y la concurrencia por defecto de Celery, 8: 160 huecos en total, más que las 128 particiones, así que cualquiera de las dos puede con un día normal ella sola. Las dos escriben en un archivo S3; las búsquedas van a Elasticsearch. Cuesta unos 4913 $ al mes, sobre todo por el archivo: unos 2569 $, porque S3 cobra cada escritura y 990 lotes por segundo son unos 2600 millones de escrituras al mes. En un día normal el p95 es de unos 152 ms.

«Un incidente triplica los logs» multiplica el tráfico por tres durante 10 segundos, a partir del segundo 20, y admite un p95 de 15 segundos. Llegan unos 1000 lotes por segundo más de los que pueden archivar 128 particiones, así que se acumulan unos 10 000 en el stream. Después, los 1000 por segundo de sobra los despachan en unos 10 segundos: el p95 queda en unos 5,4 s.

«Cae un consumidor y el grupo se rebalancea» mata la primera tarjeta Worker, Shippers A, en el segundo 20 y para siempre, en una ejecución de 120 segundos, con el mismo límite de 15 s. Los lotes que tenía A siguen asignados a ella hasta que vence el tiempo de sesión de Kafka, de 45 segundos, y luego pasan a Shippers B, que mientras tanto recibe todos los lotes nuevos. Sus 80 huecos pueden con un día normal sin ayuda, así que solo se retrasan los lotes que tenía A, y son demasiado pocos para mover el p95: se queda cerca de 0,15 s.

## Por qué falla el primer intento

Un primer intento habitual usa 96 particiones y una sola tarjeta de Shippers con 12 instancias: 96 huecos, por unos 4353 $ al mes. Supera «Un día normal» con el mismo p95. Incluso supera el incidente, aunque su atasco, mayor, se vacía más despacio y el p95 llega a unos 10,5 s.

Donde falla es en la prueba del rebalanceo. Cuando cae la única tarjeta de Shippers, el stream ya no tiene a quién entregar los lotes. A partir del segundo 20 no se archiva nada; cada lote espera hasta que termina la ejecución y cuenta con la antigüedad que tenga entonces. El p95 llega a unos 66 segundos. No se pierde ningún lote; el stream los guarda todos.

## Lo que el simulador deja fuera

Los shippers reales comprimen los logs y juntan muchos lotes en un objeto más grande, lo que reduce tanto los bytes como el número de escrituras; aquí cada lote es una escritura. La retención de los logs también está simplificada. El índice de búsqueda es un almacén aparte que leen las consultas: el simulador no indexa cada lote, así que este problema mide el camino hacia el archivo y sus consumidores.

## Para probar

- Junta las dos tarjetas de Shippers en una sola de 20 instancias y vuelve a ejecutar la prueba del rebalanceo. Los mismos huecos, el mismo coste, otro resultado.
- Mantén dos tarjetas pero baja a 96 particiones. Observa cuánto tarda en vaciarse el atasco del incidente.
- Reduce las dos tarjetas de Shippers a 6 instancias y ejecuta la prueba del rebalanceo: observa qué hace la cola de la superviviente mientras los lotes de A esperan a que venza el tiempo de sesión.
