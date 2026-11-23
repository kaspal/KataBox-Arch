# Solución comentada

## Adónde va la carga

400 clics por segundo es poco tráfico para el recolector o para Kafka. El trabajo está en el Procesador de streams, y tiene dos costes.

El primero es el procesamiento. Una ventana deslizante de 60 s que se desplaza cada 1 s actualiza 60 resultados por cada clic, así que cada clic cuesta 60 registros simples: 400 × 60 = 24 000 registros por segundo, algo más de un hueco a 20 000 por segundo. Con cuatro veces más usuarios son 96 000 registros por segundo, unos 5 huecos.

El segundo es el estado. Cada usuario con un clic en los últimos 60 s tiene una sesión abierta: 400 × 60 = 24 000 sesiones. A 128 KB cada una, son 24 000 × 128 KB ≈ 2,9 GB. Si Memoria × Instancias es menor, el estado se vuelca a disco y cada registro tarda 4 veces más. Con el volcado, esos 5 huecos se convierten en 20.

Luego está la caída. Un procesador de streams guarda su progreso (su estado y su posición en el stream) en cada checkpoint. Cuando muere, se reinicia desde el último checkpoint y procesa otra vez todo lo que llegó después, así que esos resultados llegan a la tabla de sesiones por segunda vez. El Intervalo de checkpoint fija cuánto puede ser: a 400 clics por segundo, un intervalo de 10 s repite como mucho unos 4000 resultados; uno de 600 s, hasta 240 000.

## El diseño de referencia

2 Servidores de aplicaciones Go escriben los clics en un Stream de Apache Kafka con 12 particiones. Un Procesador de streams Apache Flink con 3 instancias (4 huecos cada una, así que 12 huecos, uno por partición) lo lee con Ventana `sliding`, Duración de la ventana de 60 s, Desplazamiento de 1 s, Retraso de la marca de agua de 5 s, Estado por clave de 128 KB, Memoria de 8 GB e Intervalo de checkpoint de 10 s. Escribe cada sesión en un Almacén NoSQL ScyllaDB con "Escrituras idempotentes (dedup)" activado: una sesión se guarda con upsert por usuario, así que escribir otra vez el mismo resultado no cambia nada. Cuesta unos 1503 $ al mes: 140 $ de los Servidores de aplicaciones, 153 $ de Kafka, 210 $ de Flink y 1000 $ de ScyllaDB.

El estado de 2,9 GB cabe con holgura en 3 × 8 GB = 24 GB. Es mucho más de lo que necesita el estado de hoy, y es a propósito: con cuatro veces más usuarios, el estado real sería de unos 11,7 GB, y sigue cabiendo. Un día normal tiene un p95 de unos 5,9 s del clic a la tabla (hasta un segundo esperando el siguiente desplazamiento, más los 5 s de la marca de agua), sin errores, y el procesador está ocupado un 15 % del tiempo.

"El procesador de streams se cae" mata el Procesador de streams a los 50 segundos, durante 10 segundos, en una ejecución de 120 segundos. Los clics esperan en Kafka y no se pierde ninguno. Al reiniciarse, el procesador repite desde su último checkpoint, pero el destino deduplica, así que la ejecución no cuenta ningún duplicado. Incluso sin dedup, el checkpoint de 10 s mantiene la repetición bajo el límite: una variante de la referencia sin dedup repite unos 4000 resultados.

"Cuatro veces más usuarios" ejecuta toda la prueba a 1600 clics por segundo. El procesador trabaja unas cuatro veces más, ocupado en torno al 60 %, y el p95 se queda en unos 5,9 s sin errores.

Para el reto de coste, la búsqueda no encontró ningún diseño por debajo del 80 % del coste de la referencia (el mejor llegó al 86 %), así que este problema no tiene reto de coste. El Almacén NoSQL es la mayor parte de la factura, y el procesador ya es barato.

## Por qué falla el primer intento

Un primer intento habitual tiene la misma forma pero dimensiona el procesador como un trabajo pequeño: un Procesador de streams Apache Flink con 2 instancias de 1 GB de Memoria, el Intervalo de checkpoint en 600 s ("los checkpoints son caros, mejor pocos") y un Almacén NoSQL ScyllaDB sin dedup. Cuesta unos 1433 $ y pasa "Un día normal" con un p95 de unos 5,9 s.

Ya vuelca a disco en un día normal: 2,9 GB de estado no caben en 2 × 1 GB, así que cada registro es 4 veces más lento. El procesador está ocupado un 74 % donde la referencia está al 15 %, y todavía no se nota nada.

Con cuatro veces más usuarios necesita unas cuatro veces eso, más o menos el triple de lo que pueden hacer sus 8 huecos. Kafka guarda los clics, así que no se pierde ninguno, pero el atasco crece durante toda la prueba: el p95 sube a unos 59 s frente a un límite de 10 s (veredicto "SLO incumplido"). Darle la memoria lo arregla: el mismo diseño con 8 GB de Memoria pasa, y también uno con 3 instancias de 1 GB, porque 3 GB bastan justo para 2,9 GB.

La caída también lo rompe. El checkpoint de 600 s aún no ha llegado cuando el procesador muere a los 50 s, así que repite todo lo que ha hecho desde el principio: unos 20 300 resultados escritos dos veces en la tabla de sesiones, más del triple del límite de 6000 (veredicto "Solicitudes perdidas"). El mismo diseño con un checkpoint de 10 s repite unos 4000 y pasa.

La idea es esta: un procesador de streams con estado se dimensiona tanto por su estado como por sus registros. Calcula claves × bytes por clave, dale memoria con margen para crecer, y recuerda que un trabajo que vuelca a disco puede parecer sano hasta que sube la carga. Y el intervalo de checkpoint es cuánto trabajo estás dispuesto a rehacer tras una caída; un destino idempotente vuelve inofensivo ese trabajo repetido.

## Lo que el simulador deja fuera

El simulador no tiene ventanas de sesión. Aquí las sesiones son una ventana deslizante de 60 s en lugar de un hueco de inactividad de 30 minutos, así que el tiempo está a escala, y el número de sesiones abiertas son los clics de una ventana (como mucho, el número de usuarios). El tamaño del estado se calcula con el pico indicado de 400 clics por segundo, no con el tráfico de cada prueba, así que en "Cuatro veces más usuarios" el estado no crece; lo que crece es el trabajo, y un procesador que vuelca a disco no da abasto. El volcado es un 4 veces más lento fijo, una decisión de modelado; los backends de estado RocksDB reales empeoran de forma más gradual. Aquí los checkpoints no cuestan nada: los reales llevan tiempo, E/S y almacenamiento, y por eso los equipos no hacen un checkpoint por segundo. Los destinos transaccionales exactly-once no se simulan; el simulador cuenta los duplicados en su lugar. Una instancia del Procesador de streams cuesta lo mismo sea cual sea su Memoria, así que aquí la memoria es gratis; en un clúster real no lo es. El tiempo de recuperación tras un reinicio (restaurar el estado desde el checkpoint) no se simula.

## Para probar

- Dale a la referencia 2 instancias de 1 GB de Memoria y ejecuta "Un día normal". Pasa, pero compara la carga del procesador (un 74 %) con el 15 % de la referencia.
- Desactiva dedup en la referencia y ejecuta la caída con un Intervalo de checkpoint de 10 s, 30 s y 60 s. Mira cómo crecen los duplicados con el intervalo.
- Cambia el Desplazamiento a 5 s. El procesador hace una quinta parte del trabajo, pero cada sesión se actualiza solo cada 5 s y llegan 2000 resultados a la vez al Almacén NoSQL: el p95 sube a unos 9,9 s, justo en el límite.
