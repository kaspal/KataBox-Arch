# Solución comentada

## Adónde va la carga

30 lotes por segundo es poco para un App Server. El trabajo está en el almacén: una carga tarda unos 0,3 s, y una instancia de Snowflake ejecuta 8 a la vez, así que una instancia carga unos 8 ÷ 0,3 ≈ 27 lotes por segundo. Dos instancias (unas 53 por segundo) aguantan una hora con mucho movimiento ocupadas alrededor del 60 %.

La caída cambia la pregunta. Durante 60 s no se puede cargar nada. Lo que llega en ese minuto o falla o espera en algún sitio: 30 × 60 = 1800 lotes en una hora con mucho movimiento, 45 × 60 = 2700 en el pico. Si el App Server escribe directamente, fallan. Si se publican en un Stream, esperan: una carga que falla se reintenta, y el registro se queda en el Stream hasta que entra.

Esperar es solo la mitad. Cuando Snowflake vuelve, el atasco se vacía con la capacidad que sobra después del tráfico en vivo, no con la capacidad total, porque siguen llegando lotes nuevos:

tiempo de vaciado = atasco ÷ (ritmo de recuperación − ritmo en vivo)

En el pico, con 2 instancias: 2700 ÷ (53 − 45) ≈ 340 s, casi seis minutos de paneles desfasados tras una caída de un minuto. Con 3 instancias (80 por segundo): 2700 ÷ 35 ≈ 77 s. Con 4 (107 por segundo): 2700 ÷ 62 ≈ 44 s, dentro de los 50 s que permite el límite de recuperación (en la vida real, una caída de una hora y una recuperación de una hora más o menos; aquí a escala). El ritmo de recuperación tiene que superar al ritmo en vivo con margen, no solo superarlo.

Los reintentos añaden un retraso propio. Durante la caída, la carga de cada registro falla una y otra vez, y su espera se duplica cada vez hasta 32 s. Cuando Snowflake vuelve, los registros más antiguos regresan a lo largo de los 32 s siguientes, así que incluso con capacidad ilimitada los últimos lotes del atasco entran unos 32 s después de la vuelta.

## El diseño de referencia

Un AWS ALB envía los lotes a 2 App Servers de Go, que publican cada uno en un Stream de Apache Kafka con 48 particiones y responden enseguida al recolector. Kafka Connect, con 3 instancias y una concurrencia de 16 (48 huecos), consume el Stream y carga cada lote en Snowflake, con 4 instancias (32 cargas a la vez). Las particiones y los huecos del conector quedan por encima de los 32 del almacén, porque un registro también pasa tiempo en el conector: con solo 32 particiones, las particiones pasan a ser el tope, y siguen entrando lotes antiguos hasta los 131 s, más allá del límite.

Cuesta unos 6368 $ al mes: 5840 $ de Snowflake (1460 $ por instancia), 210 $ de Kafka Connect, 153 $ de Kafka, 140 $ de los App Servers y 25 $ del ALB. La cuarta instancia de Snowflake es el precio de la recuperación.

En un día normal la frescura p95 es de unos 0,74 s, Snowflake está ocupado alrededor del 30 % y Kafka Connect alrededor del 23 %.

"El almacén de datos cae durante 60 s" tumba Snowflake de los 20 s a los 80 s con 30 lotes por segundo, en una ejecución de 600 s. No se rechaza ni se pierde ningún lote. Unos 1800 lotes esperan en el Stream, los últimos entran hacia los 112 s (la espera de 32 s de los reintentos), y el p95 de toda la prueba es de unos 34,5 s.

"El almacén de datos cae durante 60 s en el pico de la tarde: 1,5 veces los eventos" hace lo mismo con 45 por segundo. Unos 2700 lotes esperan; el último segundo en el que los lotes que entraban habían esperado más de un minuto es el 123, y desde los 126 s vuelven a entrar en menos de 10 s. El límite de recuperación empieza a los 130 s, así que la supera, y el p95 de toda la prueba es de unos 42,9 s.

Para el reto de coste, la búsqueda no encontró ningún diseño por debajo del 80 % del coste de la referencia que supere todas las pruebas (el mejor quedó en el 99 %), así que este problema no tiene reto. La recuperación necesita unas 99 cargas por segundo en el pico, y solo 4 instancias de Snowflake las dan.

## Por qué falla el primer intento

Un primer intento habitual hace que cada App Server cargue su lote directamente en Snowflake, con 2 instancias. Cuesta unos 3085 $ y supera "Un día normal" con un p95 de unos 0,67 s, con Snowflake ocupado alrededor del 59 %.

Cuando Snowflake cae, cada carga falla al momento y nada guarda el lote: el recolector recibe un error. Durante 60 s de los 600 fallan todos los lotes, alrededor del 10,0 %, frente a un límite del 0,1 %. La prueba del pico falla igual, con un 10,1 %. El almacén volvió bien; los lotes enviados mientras no estaba se rechazaron.

El siguiente intento añade un Stream (24 particiones) y Kafka Connect (2 instancias de 12 huecos), pero mantiene 2 instancias de Snowflake, dimensionadas para la hora con mucho movimiento. Cuesta unos 3378 $. En una hora con mucho movimiento supera la caída: 1800 lotes se vacían a 53 − 30 = 23 por segundo, en unos 78 s, y el p95 es de unos 52 s. En el pico falla: la capacidad sobrante es de 53 − 45 = 8 por segundo, los 2700 lotes necesitan unos 340 s, y los lotes que entraban seguían habiendo esperado más de un minuto hasta los 167 s y más de 50 s hasta los 226 s más o menos. No se pierde nada, pero los paneles siguen minutos por detrás. Con 3 instancias (4908 $) el vaciado tarda unos 77 s, y siguen entrando lotes que esperaron más de un minuto hasta los 138 s, más allá del límite de los 130 s.

La idea es esta: un búfer convierte una caída en un retraso, y la capacidad sobrante decide cuánto dura ese retraso. Dimensiona la recuperación para el pico, con margen suficiente sobre el ritmo en vivo para que el atasco de toda una caída se vacíe en el tiempo que te puedes permitir.

## Lo que el simulador deja fuera

Cada lote es una carga de unos 0,3 s. Las tuberías reales ajustan el tamaño del lote y el número de ficheros por carga (Snowpipe, COPY, el búfer de un conector de destino), y los lotes más grandes durante una recuperación cargan más rápido por evento; eso no se modela. Aquí las instancias de Snowflake son fijas: un almacén multiclúster real puede añadir clústeres mientras dura el atasco y quitarlos después, lo que hace que la capacidad de recuperación salga más barata que pagarla todo el mes. Subir la Concurrencia por instancia no hace más lentas las cargas aquí, mientras que las cargas reales en un clúster comparten su capacidad de cómputo; por eso el enunciado la fija en 8. La espera entre reintentos (1 s, que se duplica hasta 32 s) es la del simulador; los ajustes propios de reintento y tolerancia a errores de Kafka Connect, sus offsets, los rebalanceos y los temas de mensajes fallidos no se simulan. Los duplicados de una carga reintentada que en realidad había funcionado, los cambios de esquema y la corrección de los propios datos no se simulan. El tiempo está a escala: la caída dura 60 s y la ventana de recuperación 50 s. El precio de Kafka Connect es un precio didáctico, no una cotización.

## Para probar

- Pon Snowflake en 3 instancias, ejecuta la caída en el pico y compara cuándo dejan de entrar los lotes antiguos con los 77 s que da la fórmula de vaciado.
- Pon Snowflake en 6 instancias. Los lotes antiguos siguen entrando hasta los 114 s más o menos: casi todo eso es la espera de los reintentos, no el almacén.
- Pon el Stream en 32 particiones y mantén 4 instancias de Snowflake. ¿Qué tarjeta es el tope ahora?
