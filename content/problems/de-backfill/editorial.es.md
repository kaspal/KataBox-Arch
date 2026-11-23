# Solución comentada

## Adónde va la carga

Cada lote, en vivo o del relleno, acaba en una carga en el almacén, y ese es el único paso caro. Una carga ocupa un hueco del almacén unos 300 ms de mediana y más en la cola lenta, así que una instancia de Snowflake con 8 huecos carga como mucho unos 8 / 0,3 ≈ 27 lotes por segundo, y unos 24 contando las cargas lentas. La API de ingesta, el Stream y el Procesador de streams son baratos y van muy desahogados.

En el pico normal, 24 lotes en vivo y 16 del relleno por segundo piden 40 cargas por segundo. Dos instancias (unas 48 por segundo) pueden con ello, ocupadas alrededor del 86 %. Las pruebas ocultas se comen ese margen tan fino de dos maneras:

- A 5 veces su ritmo, el relleno envía 80 lotes por segundo: 104 en total, más del doble de lo que cargan dos instancias.
- En el pico en vivo llegan 60 lotes en vivo por segundo por sí solos, más de lo que dos instancias pueden cargar aunque no hubiera relleno.

Cuando el pipeline pide más cargas de las que el almacén puede hacer, se forma un montón en el Stream, y el orden en que el Stream lo reparte decide quién espera. Un lote en vivo se mide por finalización (hasta que está en el almacén), así que un lote en vivo que espera deja los paneles desfasados. Un lote del relleno se mide por respuesta (hasta que el Stream lo tiene), así que un lote del relleno que espera no cuesta nada: se confirmó en cuanto quedó guardado. Es la misma división que usa la repetición de una caída "El atasco que enterró los pedidos"; allí el montón venía de una degradación imprevista del almacenamiento, aquí de un trabajo planificado al que alguien le subió el ritmo.

## El diseño de referencia

Un AWS ALB envía los lotes a 2 Servidores de aplicaciones Go, que los publican en un Stream Apache Kafka con 32 Particiones y "Primero lo urgente (prioridad)" activado. Un Procesador de streams Apache Flink con 2 instancias y una Concurrencia por instancia de 16 lo consume y carga cada lote en un Almacén de datos Snowflake con 3 Instancias. Cuesta unos 4838 $ al mes: 4380 $ el almacén, 153 $ Kafka, 140 $ Flink, 140 $ los Servidores de aplicaciones y 25 $ el ALB. Tres instancias cargan unos 72 lotes por segundo, y las 32 particiones y los 32 huecos de Flink bastan para mantener ocupados los 24 huecos del almacén. Una hora punta normal tiene un p95 de unos 584 ms, con el almacén ocupado alrededor del 57 %.

"El relleno corre a 5 veces su ritmo, en el pico" multiplica el relleno por 5 durante todo el minuto. Al pipeline se le piden 104 cargas por segundo y hace unas 72, así que el relleno se acumula en el Stream a razón de unos 30 lotes por segundo. Con la prioridad activada, cada lote en vivo nuevo pasa por delante de todos los lotes del relleno en cola, así que los lotes en vivo solo esperan a que quede libre un hueco. El p95 es de unos 573 ms, sin errores y sin nada perdido: los lotes del relleno se guardan y se confirman, y esperan su turno.

"Un pico en vivo: 2,5 veces los eventos en vivo" envía 60 lotes en vivo por segundo con el relleno a sus 16 normales. Los 60 lotes en vivo caben en las 72 cargas por segundo con margen, y el relleno se lleva lo que sobra y espera por el resto. El p95 es de unos 818 ms.

Los dos arreglos cubren pruebas distintas. La prioridad sola, con las dos instancias del primer intento, supera el relleno rápido (p95 de unos 479 ms) pero no el pico: 60 lotes en vivo por segundo son más de lo que cargan dos instancias, y los lotes en vivo se acumulan unos detrás de otros (p95 de unos 15,9 s). Tres instancias sin prioridad superan el pico (p95 de unos 5,2 s) pero no el relleno rápido (p95 de unos 20,6 s). Cuatro instancias sin prioridad superan las tres pruebas por 6298 $, pero solo porque la prueba se acaba: 104 lotes por segundo contra unas 96 cargas por segundo siguen siendo un montón que crece, y los lotes en vivo están dentro. Cinco instancias se pasan del presupuesto.

Este problema no tiene reto de coste. Las instancias del almacén son alrededor del 90 % de la factura, y el diseño más barato que encontró la búsqueda y que supera todas las pruebas cuesta el 99 % de la referencia.

## Por qué falla el primer intento

Un primer intento habitual es el mismo pipeline dimensionado para el pico normal: un Stream Kafka con 16 Particiones y sin prioridad, Flink con 2 instancias y una Concurrencia por instancia de 8, y un almacén Snowflake con 2 Instancias. Cuesta unos 3378 $ y supera la hora punta normal con un p95 de unos 632 ms, con el almacén ocupado alrededor del 86 %.

Cuando el relleno corre a 5 veces su ritmo, llegan 104 lotes por segundo y se cargan unos 48. El Stream es FIFO, así que cada lote en vivo nuevo se pone al final de un montón que crece unos 55 lotes por segundo. Un lote en vivo espera más de 10 s a partir de los 19 s de prueba, más o menos, y más de 30 s al final. El p95 es de unos 41,3 s. Nada falla y nada se pierde: todos los lotes siguen en el Stream. Simplemente, los paneles van desfasados.

El pico en vivo también lo rompe: 76 lotes por segundo contra 48, el mismo montón que crece, un p95 de unos 33,8 s. Aquí la prioridad sola no habría bastado: los lotes en vivo por sí solos superan lo que cargan dos instancias.

La idea es esta: un relleno que comparte el pipeline en vivo nunca debe ir por delante del trabajo en vivo, y el pipeline debe tener sitio para el pico en vivo por sí solo. Deja que los registros en vivo vayan primero, que el relleno espere en el Stream, donde esperar es barato, y dimensiona la etapa cara para el pico en vivo, no para la media.

## Lo que el simulador deja fuera

Kafka no tiene prioridad de mensajes. "Primero lo urgente (prioridad)" representa lo que los equipos construyen en su lugar: dos topics, con el procesador leyendo siempre primero el de en vivo, o un pipeline aparte (su propio Stream, su procesador y su clúster del almacén) para el relleno. Los dos tipos de lote de este problema comparten una ruta, así que las pruebas juzgan solo el pipeline compartido. Los almacenes reales cargan micro-lotes a través de archivos (Snowpipe, COPY) y cobran el tiempo de cómputo; aquí una carga es un hueco ocupado unos 300 ms, y una instancia se cobra por meses. No se juzga la finalización del propio relleno, y la prueba no comprueba que llegue a terminar; no se simula leer el histórico del archivo, ni sobrescribir las filas antiguas con las corregidas (ni los duplicados que dejaría repetir el relleno). Aquí un Stream guarda hasta 100 000 lotes en espera; por encima de eso, rechaza las publicaciones. Un relleno real dura días; cada prueba ve un minuto de él.

## Para probar

- Desactiva la prioridad en la referencia y ejecuta el relleno rápido. Mira cómo los lotes en vivo esperan detrás del montón del relleno.
- Activa la prioridad en el primer intento y ejecuta el pico en vivo. Compara 60 lotes en vivo por segundo con lo que pueden cargar dos instancias del almacén.
- Dale a la referencia 32 huecos de Flink pero solo 16 Particiones. ¿Qué límite decide cuántas cargas van a la vez?
