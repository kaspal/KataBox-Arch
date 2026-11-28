# Solución comentada

## Adónde va la carga

De 1000 peticiones por segundo, 800 son lecturas en línea, 198 son actualizaciones de características y 2 son lecturas de entrenamiento. Cada una pide un almacén distinto.

Un almacén de datos dedica unos 300 ms a cada consulta, y una instancia ejecuta 8 a la vez, así que una instancia responde como mucho unas 8 / 0,3 ≈ 27 consultas por segundo. Servir con él las 800 lecturas en línea mantendría 800 × 0,3 = 240 consultas en curso: 30 instancias, unos 44 000 $ al mes, y aun así un p95 de 300 ms como poco frente a un objetivo de 50 ms. Las 2 lecturas de entrenamiento por segundo son otra cosa: cada una tarda unos 0,8 s (300 ms más 0,5 GB a 1 GB/s), y juntas recorren 2 × 0,5 = 1 GB por segundo. Las instancias de un almacén de datos reparten su velocidad de escaneo entre las consultas que se ejecutan a la vez, así que eso es todo el 1 GB/s de una instancia, y unas rebajas que triplican el tráfico necesitan 3 GB/s: tres instancias. Ese es el trabajo para el que está hecho un almacén de datos.

Las actualizaciones son una cuestión de frescura. En la referencia, cada una espera solo unos milisegundos a su escritura, así que unos pocos huecos siguen el ritmo de 198 por segundo. La ralentización oculta suma 100 ms a cada registro: entonces tiene que haber 198 × 0,1 ≈ 20 registros en curso a la vez, y ese número tiene como tope las particiones del Stream o los huecos del procesador de streams, lo que sea menor. Por debajo de 20, el Stream guarda lo que sobra y el retraso crece cada segundo.

## El diseño de referencia

Un AWS ALB envía el tráfico a 2 App Servers Go y a un microservicio Genérico. Para una predicción, el App Server lee la fila del usuario de un almacén NoSQL ScyllaDB con 1 partición: el almacén en línea, que tiene a todos los usuarios. Para un evento, lo publica en un Stream Apache Kafka con 32 particiones. Apache Flink, 4 instancias con una concurrencia de 8 (32 huecos), consume el Stream y escribe las nuevas características de cada usuario en el mismo ScyllaDB. El microservicio, el servicio de características offline, envía las lecturas de entrenamiento a un almacén de datos Snowflake con 3 instancias. Nada en línea toca el almacén de datos.

Cuesta unos 5578 $ al mes: 4380 $ de Snowflake, 560 $ de Flink, 250 $ de ScyllaDB, 153 $ de Kafka, 140 $ de los App Servers, 70 $ del microservicio y 25 $ del ALB. Snowflake solo cobra mientras está en marcha, pero una lectura de entrenamiento cada medio segundo lo mantiene en marcha todo el mes. En un día normal, el p95 es de unos 23,2 ms, sin errores; ScyllaDB y Flink están ocupados un 4 % más o menos, y el almacén de datos usa alrededor de un tercio de su velocidad de escaneo.

«Unas rebajas: el triple de tráfico durante 30 s» triplica todo a partir del segundo 20, en una ejecución de 90 segundos. ScyllaDB responde a cada usuario en pocos milisegundos, lleguen los usuarios distintos que lleguen, y a Flink le sobran huecos. Las tres instancias del almacén de datos recorren los 3 GB por segundo de las lecturas de entrenamiento. El p95 se queda en unos 23,2 ms, sin errores. Con 2 instancias, las lecturas de entrenamiento hacen cola durante las rebajas y falla alrededor del 0,08 % de las peticiones, demasiado cerca del límite del 0,1 %; con 1, falla alrededor del 0,15 % incluso en un día normal.

«El procesador de streams se ralentiza durante 30 s: las características deben llegar igualmente en 1 s» suma 100 ms a cada registro de Flink a partir del segundo 20, durante 30 segundos. Como cada actualización va más lenta a propósito, el límite de p95 de esta prueba es 1 s en vez de 50 ms: lo que comprueba es que la cola pendiente no crezca. Con 32 huecos y 32 particiones, Flink necesita unos 20 registros en curso y aún le sobra sitio, así que cada actualización simplemente tarda unos 100 ms más. El p95 es de unos 112,9 ms, sin errores y sin actualizaciones perdidas. Las 4 instancias están ahí por esta prueba: en un día normal Flink casi no trabaja.

Este problema no tiene reto de coste. El almacén de datos es la mayor parte de la factura, y los diseños más baratos que encuentra la búsqueda solo cambian Snowflake por otro producto.

## Por qué falla el primer intento

Un primer intento habitual deja las características donde ya están, en el almacén de datos, y pone una caché delante. Los App Servers miran en una caché Redis con 4 GB de memoria y un TTL de 3600 s (una hora), y en un fallo leen de un almacén de datos Snowflake con 2 instancias. Flink, con sus valores por defecto (2 instancias, una concurrencia de 4), escribe cada actualización directamente en Redis, y Kafka tiene sus 6 particiones por defecto. Cuesta unos 3497 $: 2920 $ son las dos instancias del almacén de datos.

Supera «Un día normal» con un p95 de unos 23,2 ms. Los usuarios activos están siempre en Redis, y los fallos son tan pocos que el almacén de datos los aguanta, aunque ya está ocupado un 84 % más o menos.

Las rebajas lo rompen: un 40,2 % de errores y un p95 de unos 1572 ms. El triple de tráfico trae muchos más usuarios que no están en la caché, y cada una de sus lecturas se convierte en una consulta de 300 ms al almacén de datos. Dos instancias ejecutan 16 a la vez; el resto espera, la cola se llena y las lecturas que vienen detrás fallan. Una tercera instancia no lo salva: el diseño pasa a costar unos 4957 $ y aun así falla un 25,5 % de las peticiones en las rebajas.

La ralentización también lo rompe, por una segunda razón: un p95 de unos 5924 ms frente al límite de 1 s. Con 6 particiones, Flink solo puede tener 6 registros en curso; a unos 100 ms cada uno, son unas 60 actualizaciones por segundo frente a las 198 que llegan. El Stream guarda el resto y no se pierde nada, pero el retraso crece unos 0,7 s cada segundo, hasta unos 20 s al final de la ralentización.

La idea es esta: las lecturas en línea y las de entrenamiento son cargas distintas. Servir en línea necesita un almacén que tenga las características de todos los usuarios y responda cualquiera de ellas en milisegundos, mantenido al día por el stream a medida que llegan los eventos. El almacén de datos es donde el entrenamiento lee el histórico; una caché delante solo oculta su latencia para los usuarios que ha visto hace poco. Y un camino de streaming necesita margen en particiones y huecos, para que un minuto lento no se convierta en un retraso que crece.

## Lo que el simulador deja fuera

No se simulan la carga diaria en el almacén de datos ni la primera copia de cada usuario en el almacén en línea; las pruebas empiezan con las dos hechas. No se simula la corrección en el tiempo de las lecturas de entrenamiento (leer cada característica como era en el momento de la etiqueta, para que un modelo nunca entrene con el futuro), ni las diferencias entre las características usadas al entrenar y al servir. Cada evento es una actualización de la fila de un usuario; aquí no se modelan las agregaciones por ventanas de Flink ni su estado. No se comprueban los valores de las características, solo cuándo llegan. No se simula la replicación de ScyllaDB entre nodos, y los precios de ScyllaDB y Flink son cifras didácticas del registro que aún necesitan una fuente.

## Para probar

- Dale al primer intento el camino de streaming de la referencia (32 particiones, Flink con 32 huecos). ¿Qué prueba oculta sigue fallando, y por qué?
- Pon Flink en 2 instancias (16 huecos) en la referencia y ejecuta la ralentización. Con unos 20 registros necesarios en curso, mira el retraso.
- Baja Kafka a 6 particiones y deja los 32 huecos de Flink. ¿Cuál de los dos topes manda ahora?
