# Solución comentada

## Adónde va la carga

40 eventos por segundo es poco trabajo para el servicio de ingesta y para Kafka. El trabajo son las escrituras: cada evento se convierte en una fila del almacén, y una escritura tarda unos 300 ms. A 40 eventos por segundo son 40 × 0,3 = 12 escrituras en curso de media; en la tormenta de reintentos, a 120 eventos por segundo, son 36, más el atasco que deja la caída. Un hueco del Procesador de streams espera a su escritura, así que el procesador necesita esos huecos, el Stream esas particiones (un evento por partición a la vez) y el almacén esos puestos de escritura (8 por nodo).

Luego está la caída, y el único número que no puede ser otro que cero. Un procesador de streams guarda su progreso (su posición en el stream y su estado) en cada checkpoint. Cuando muere, se reinicia desde el último checkpoint y vuelve a procesar todo lo posterior. Esos eventos ya se escribieron una vez, así que sin protección llegan al libro por segunda vez: un cliente cobrado dos veces. Un Intervalo de checkpoint más corto reduce lo que se repite, pero nunca llega a cero, porque la caída puede llegar un instante después de cualquier checkpoint. Solo el destino puede hacer inofensiva la repetición: una escritura con el id del evento como clave (un upsert, o el almacén descartando una fila que ya tiene) convierte la entrega "al menos una vez" en un efecto exactamente una vez.

## El diseño de referencia

2 servidores de aplicación en Go escriben los eventos en un Stream Apache Kafka con 64 particiones. Un Procesador de streams Apache Flink con 2 instancias y Concurrencia por instancia 32 (64 huecos, uno por partición) lo lee, con Ventana `none` e Intervalo de checkpoint 10 s. Escribe cada evento en un Almacén de datos ClickHouse con 8 instancias (64 escrituras a la vez, unas 210 por segundo) y "Escrituras idempotentes (dedup)" activado. Cuesta unos 2433 $ al mes: 140 $ los servidores de aplicación, 153 $ Kafka, 140 $ Flink y 2000 $ ClickHouse.

En un día normal, el p95 del evento al libro es de unos 0,7 s, y el procesador y el almacén están ocupados cada uno en torno al 21 %.

"El procesador de streams se cae" mata el Procesador de streams a los 50 segundos, durante 10 segundos, en una ejecución de 120 segundos. Los eventos esperan en Kafka y no se pierde nada. Al reiniciarse, el procesador repite desde su último checkpoint y vuelve a escribir esos eventos, pero el almacén los deduplica, así que el libro no cuenta duplicados. El p95 sube a unos 6,2 s mientras se vacía el atasco.

"Una tormenta de reintentos" envía 3 veces los eventos (120 por segundo) de los 30 s a los 60 s, y mata el procesador a los 45 s, en medio. La repetición es mayor, y de nuevo el destino la absorbe: ningún duplicado. El atasco de la tormenta y la caída juntas tarda en vaciarse, y el p95 llega a unos 21,9 s, por debajo del límite de 30 s. Un diseño más pequeño, con 6 nodos de almacén, 48 huecos y 48 particiones, aguanta un día normal y la caída, pero no esto: su p95 sube a unos 40 s.

El Intervalo de checkpoint de 10 s no cambia aquí la cuenta de duplicados (dedup la deja en cero con cualquier intervalo). Fija cuánto trabajo rehace una caída, y en un clúster real cuánto tarda la recuperación y cuánto cuesta un checkpoint; 10 s es un término medio habitual.

Para el reto de coste, la búsqueda encontró un diseño a un 53 % del coste de la referencia (1293 $): un servidor de aplicación, una instancia de Flink con 32 huecos y 4 nodos de ClickHouse, todavía con dedup. El objetivo es el 55 % del coste de la referencia. Funciona porque la tormenta dura solo 30 s y Kafka guarda el atasco; tiene mucho menos margen para una tormenta más larga.

## Por qué falla el primer intento

Un primer intento habitual tiene la misma forma y la misma capacidad, y conoce las caídas: "haz checkpoints a menudo y se repetirá poco". Pone el Intervalo de checkpoint en 5 s y deja "Escrituras idempotentes (dedup)" desactivado. Cuesta los mismos 2433 $ y pasa "Un día normal" con un p95 de unos 0,7 s: sin caída no hay nada que repetir.

En "El procesador de streams se cae", el procesador se reinicia desde un checkpoint de hace unos segundos y vuelve a escribir esos eventos: unas 186 filas dos veces en el libro (veredicto "Solicitudes perdidas"; el límite es 0). En la tormenta de reintentos el procesador está más ocupado cuando muere, así que esos mismos segundos contienen más eventos: unos 599 duplicados.

Hacer checkpoints más a menudo no lo arregla. Con 1 s, la caída sigue dejando unos 32 duplicados y la tormenta unos 115. Con 60 s (el valor por defecto) son unos 1955 y 2954, porque la caída llega antes del primer checkpoint y se repite todo desde el principio. El intervalo decide a cuántos clientes se cobra dos veces, nunca si ocurre.

La idea es esta: un procesador de streams con checkpoints entrega cada registro al menos una vez, y tras una caída algunos más de una vez. "Exactamente una vez" no es una propiedad de la entrega sino del efecto: haz idempotente el destino, con un id que el evento lleva desde el principio como clave, y las repeticiones no cambian nada. Después dimensiona el pipeline para que la repetición y la tormenta se vacíen a tiempo.

## Lo que el simulador deja fuera

El simulador no sigue los ids de los eventos. En el mundo real, los reintentos de los productores reenvían el mismo evento, y esos reenvíos también son duplicados; el mismo destino idempotente, con el id del evento como clave, los absorbe. Aquí una tormenta de reintentos se modela honestamente con lo que el simulador puede contar: tres veces el tráfico (más carga) más una caída durante ella. Los duplicados solo se cuentan en la repetición de un procesador de streams tras una caída. Los destinos transaccionales exactamente una vez (el commit en dos fases de Flink, las transacciones de Kafka) no se simulan; aquí dedup en el destino es la única forma de llegar a cero. Dedup se modela gratis: los upserts reales o los motores de tabla que deduplican cuestan algo de velocidad de escritura y almacenamiento, y el ReplacingMergeTree de ClickHouse elimina las repeticiones solo cuando fusiona partes. Los checkpoints no cuestan nada aquí, y la recuperación tras un reinicio (restaurar el estado, rebobinar el stream) no lleva tiempo. Escribir una fila por evento en un almacén es una simplificación; los pipelines reales agrupan sus escrituras, lo que cambia el dimensionado pero no la lección.

## Para probar

- Desactiva dedup en la referencia y ejecuta la caída con un Intervalo de checkpoint de 1 s, 10 s y 60 s. Los duplicados bajan con el intervalo, pero nunca llegan a cero.
- Dale a la referencia 6 nodos de almacén, 48 huecos y 48 particiones y ejecuta la tormenta de reintentos. Pasa la caída, pero el atasco de la tormenta lleva el p95 a unos 40 s.
- Construye el diseño del reto (un servidor de aplicación, una instancia de Flink con 32 huecos, 4 nodos de ClickHouse) y mira cuánto se acerca su p95 en la tormenta al límite.
