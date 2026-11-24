# Solución comentada

## Adónde va la carga

12 cambios por segundo es muy poco para PostgreSQL. Lo difícil está en el otro extremo: el almacén escribe cada cambio por separado, en unos 300 ms de mediana y algo más de media (unos 0,34 s, por la cola lenta). Un cambio que espera su escritura mantiene algo ocupado todo ese tiempo, así que el número de escrituras en curso es el ritmo por el tiempo de escritura: 12 × 0,34 ≈ 4 en un día normal.

Las pruebas ocultas suben ese número de dos maneras. La venta flash triplica el ritmo: 36 × 0,34 ≈ 12 escrituras en curso. El almacén lento hace que cada escritura tarde unos 1,84 s: 12 × 1,84 ≈ 22 escrituras en curso, al ritmo normal.

Tres tarjetas ponen tope a las escrituras en curso, y manda el tope más pequeño:
- el Stream entrega un registro por partición cada vez, así que las particiones son un tope;
- el procesador de streams retiene un hueco (instancias × concurrencia) durante toda la escritura;
- el almacén de datos ejecuta 8 escrituras por instancia (su concurrencia).

Lo que la tubería mueve en un segundo es ese tope más pequeño dividido por el tiempo de escritura. Cuando los cambios llegan más deprisa, el Stream los guarda en orden y no se pierde nada, pero cada uno espera más, y el límite de frescura de 15 s (5 minutos en la vida real, a escala para las pruebas cortas del simulador) es lo que puede crecer esa espera.

La captura de cambios es lo que lo hace posible. Una exportación que lee la tabla de pedidos cada hora, o cada noche, dejaría el almacén hasta ese intervalo por detrás y leería la tabla entera de la base de datos de la tienda cada vez. El registro de cambios ya tiene cada cambio confirmado, en orden, en el momento en que ocurre.

## El diseño de referencia

Un AWS ALB envía el tráfico a 2 App Servers Go, que escriben cada cambio en PostgreSQL. PostgreSQL está conectado con una conexión Publish (captura de cambios) a un Stream Apache Kafka con 24 particiones. Apache Flink, 2 instancias con una concurrencia de 12 (24 huecos), consume el Stream y escribe cada cambio en Snowflake, con 3 instancias (24 escrituras a la vez). Los tres topes son iguales: 24 escrituras en curso.

Cuesta unos 5378 $ al mes: 4380 $ de Snowflake (1460 $ por instancia), 260 $ de PostgreSQL, 153 $ de Kafka, 420 $ de Flink, 140 $ de los App Servers y 25 $ del ALB. El almacén es casi todo.

En un día normal, el p95 de frescura es de unos 0,70 s, y el procesador de streams está ocupado un 17 % más o menos: 4 escrituras en curso de 24.

"Una venta flash: 3 veces los cambios durante 30 s" envía 36 cambios por segundo a partir del segundo 20, en una ejecución de 90 segundos. 24 escrituras en curso mueven unos 24 / 0,34 ≈ 70 cambios por segundo, así que el pico cabe, y el p95 se queda en unos 0,68 s.

"El almacén de datos se ralentiza: cada escritura tarda 1,5 s más durante 30 s" hace que cada escritura tarde unos 1,84 s a partir del segundo 20. 24 escrituras en curso siguen moviendo unos 24 / 1,84 ≈ 13 cambios por segundo, justo por encima de los 12 que llegan. El p95 sube a unos 2,4 s, las escrituras de la tienda no lo notan y no se pierde ningún cambio.

Para el reto de coste, un diseño de prueba se queda en el 68 % del coste de la referencia (3638 $), por debajo del objetivo del 70 %. Mantiene 24 particiones, pero usa 1 App Server, 1 instancia de Flink con una concurrencia de 12 y 2 instancias de Snowflake. Durante la ralentización mueve menos cambios de los que llegan, y el Stream guarda la diferencia; después la cola se vacía lo bastante rápido para quedarse por debajo de 15 s en el p95.

## Por qué falla el primer intento

Un primer intento habitual tiene las mismas tarjetas con sus valores por defecto: Kafka con 6 particiones, Flink con 2 instancias de 4 huecos (8) y Snowflake con 1 instancia (8 escrituras). Cuesta unos 2178 $ y supera "Un día normal" con un p95 de unos 0,80 s. Las particiones son el tope más pequeño: 6 escrituras en curso mueven unos 6 / 0,34 ≈ 18 cambios por segundo, y un día normal necesita 12.

La venta flash necesita 36 por segundo. Durante 30 segundos llegan unos 18 cambios por segundo más de los que se escriben, así que se acumulan unos 540 en el Stream, y cuando acaba la venta se vacían solo a los 6 por segundo que le sobran a la tubería. El p95 de frescura llega a unos 23,7 s, por encima del límite de 15 s.

La ralentización es la misma historia desde el otro lado. Con escrituras de 1,84 s, 6 en curso mueven solo unos 3,3 cambios por segundo, mientras siguen llegando 12. Se acumulan unos 260 cambios en 30 segundos, y el p95 llega a unos 22,3 s.

En ninguna de las dos pruebas se pierde nada, y la tienda nunca va más lenta: el Stream hace su trabajo. Lo que falla es la frescura. La idea es esta: el rendimiento de una tubería que escribe en un destino lento son sus escrituras en curso divididas por el tiempo de escritura. Dimensiona juntas las particiones, los huecos del procesador y las escrituras del almacén, para la escritura más lenta que tengas que aguantar, no para el día medio.

## Lo que el simulador deja fuera

El almacén escribe cada cambio por separado. Las tuberías reales agrupan muchos cambios en una sola carga (Snowpipe, COPY, los microlotes de un conector de destino), que necesita muchas menos escrituras en curso; la lección del tope más pequeño se aplica entonces por lote en lugar de por cambio. Debezium no es una tarjeta aparte: la conexión Publish de la base de datos envía cada cambio confirmado al instante, y los reinicios del conector, sus offsets y la instantánea inicial de una tabla grande no se simulan. Tampoco se simulan los cambios de esquema, la fusión de actualizaciones y borrados en una sola fila ni la entrega exactamente una vez; ninguna prueba hace caer al procesador de streams, así que no aparecen duplicados. Snowflake cobra los segundos que está ocupado, alargados por su suspensión automática, y una tubería que escribe todo el día lo mantiene ocupado, así que aquí cada instancia cuesta el mes entero. Más instancias se modelan como más escrituras a la vez a la misma velocidad; las consultas que compiten por el cómputo de un mismo almacén no se modelan. El precio de Flink es un precio didáctico, no una cotización.

## Para probar

- Vuelve a poner Kafka en 6 particiones y deja todo lo demás igual. Ejecuta la venta flash y mira qué tarjeta es ahora el tope más pequeño.
- Deja Snowflake con 1 instancia y ejecuta la ralentización. Compara 8 escrituras en curso con las 22 que necesita el almacén lento.
- Vuelve a poner Flink con sus valores por defecto (8 huecos), mantén 24 particiones y 3 instancias de Snowflake, y ejecuta otra vez la ralentización.
