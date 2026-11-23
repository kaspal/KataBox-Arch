# Solución comentada

## Adónde va la carga

Con 2000 peticiones por segundo, el 97 % son eventos nuevos: unos 1940 por segundo, cada uno una escritura en el índice de búsqueda. Las búsquedas son solo 60 por segundo. Así que el índice pasa casi todo su tiempo indexando, y la pregunta es qué pasa cuando no da abasto.

Las pruebas ocultas preguntan justo eso. Cuando un proceso por lotes lo registra todo de golpe, el tráfico se triplica durante 20 segundos: unos 5800 eventos por segundo, unos 116 000 eventos en total. Cuando el clúster de búsqueda cae durante 30 segundos, llegan unos 58 000 eventos que no tienen dónde escribirse. Si el App Server escribe en el índice él mismo, cada uno de esos eventos espera al índice o falla. Si el evento está antes a salvo en otro sitio, se puede indexar más tarde.

## El diseño de referencia

Un Client llama a un AWS ALB delante de 2 App Servers Go. Por cada evento, el App Server publica en un Stream Apache Kafka con 128 Particiones y responde en el acto. 6 instancias de indexadores Celery, cada una con una Concurrencia por instancia de 32 y un Tiempo de servicio de 5 ms, consumen el stream y escriben en OpenSearch con 6 Shards. Las búsquedas van directamente de los App Servers a OpenSearch. Cuesta unos 1488 $ al mes: OpenSearch 750 $, los indexadores 420 $, Kafka 153 $, los App Servers 140 $ y el ALB 25 $.

En «Un día normal» el p95 es de unos 24 ms, sin errores. OpenSearch está ocupado en torno al 61 % y los indexadores en torno al 33 %.

¿Por qué 128 particiones? Un stream entrega cada partición a un solo consumidor a la vez. Con las 6 por defecto, solo se indexan 6 eventos a la vez, y los indexadores solo podrían con unos pocos cientos de eventos por segundo, muy por debajo de 1940. Con 128, los 192 huecos de los indexadores tienen trabajo suficiente que repartirse.

«Un proceso por lotes lo registra todo de golpe» triplica el tráfico a partir del segundo 15, durante 20 segundos: unos 6000 eventos por segundo. Los App Servers solo publican, así que la respuesta no espera al índice: el p95 es de unos 25 ms, sin errores. El stream guarda lo que el índice todavía no puede aceptar, y los indexadores se ponen al día después de la ráfaga.

«El clúster de búsqueda cae durante 30 s» detiene OpenSearch a partir del segundo 20, en una ejecución de 120 segundos. Admite un 5 % de errores y un 95 % de disponibilidad. Las búsquedas durante la caída fallan, y eso no lo arregla nada: son el 3 % del tráfico durante una cuarta parte de la ejecución, en torno a un 0,78 % de errores en total. No se pierde ningún evento. Kafka guarda cada registro, y los indexadores reintentan los que fallaron hasta que OpenSearch vuelve: un stream no descarta un registro con el que su consumidor falló.

Para el reto de coste, un diseño de prueba se queda en el 76 % del coste de la referencia, por debajo del objetivo del 80 %. Mantiene la misma forma, con menos App Servers y menos instancias de indexadores.

## Por qué falla el primer intento

Un primer intento habitual son 4 App Servers Go que escriben cada evento en OpenSearch en línea, con 6 Shards, y una tarjeta Celery que se queda en el lienzo sin conectar. Cuesta unos 1475 $, casi lo mismo que la referencia, y supera «Un día normal» con un p95 de unos 65 ms.

En el proceso por lotes llegan unos 6000 eventos por segundo, y OpenSearch no puede indexar tantos. Cada petición espera a su escritura, las esperas se acumulan y muchas agotan su tiempo: un 26,8 % de errores, con un p95 de unos 786 ms. Cuando el clúster de búsqueda cae, se rechaza cada evento de esos 30 segundos, porque no hay otro sitio donde ponerlo: un 25 % de errores, muy por encima del 5 % que admite la prueba.

La idea es esta: confirma un evento en cuanto es duradero en un log, e indéxalo desde el log al ritmo del propio índice. El stream absorbe tanto una ráfaga como una caída, y el coste es el mismo.

## Lo que el simulador deja fuera

No se simulan la replicación de Kafka ni su configuración de acks: un evento es duradero en cuanto se publica. Tampoco la retención: los registros se guardan durante toda la prueba. También quedan fuera el orden dentro de una partición, la indexación por lotes de OpenSearch, y la inmutabilidad y la protección contra manipulaciones que necesita un registro de auditoría real (cadenas de hashes, almacenamiento WORM).

## Para probar

- Vuelve a poner las Particiones del Stream en 6 y ejecuta «Un día normal». Mira cuánto se ocupan los indexadores y cuánto se retrasa la indexación.
- Quita el Stream y conecta los App Servers directamente a OpenSearch, y después ejecuta el proceso por lotes. Compara los errores con los de la referencia.
- Reduce los indexadores a 2 instancias y ejecuta «El clúster de búsqueda cae durante 30 s». Mira cuánto tarda en vaciarse el atasco cuando OpenSearch vuelve.
