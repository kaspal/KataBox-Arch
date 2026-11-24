# Solución comentada

## Adónde va la carga

25 000 contadores envían una lectura cada 10 segundos: 2500 lecturas por segundo, cada una una escritura en la Base de datos de series temporales. Un nodo admite 4000 puntos por segundo, así que en un día normal basta con un nodo, a un 62 % de su capacidad. Las 25 000 conexiones MQTT son fáciles para el Broker IoT: dos instancias de EMQX mantienen hasta 2 000 000.

Las pruebas ocultas aprietan los dos límites del nodo de la base de datos. Cuando la flota se reconecta tras un corte de red, las lecturas llegan 4 veces más rápido durante 20 segundos (desde el segundo 20): 10 000 por segundo, dos veces y media lo que escribe un nodo. Cuando la actualización de firmware añade una etiqueta, las 250 000 series pasan a 2 500 000 durante 20 segundos. Un nodo indexa 1 000 000; por encima de 2 000 000 rechaza las escrituras.

Así que las preguntas son: ¿dónde esperan las lecturas cuando llegan más rápido de lo que escribe la base de datos, y cuántos nodos necesitan las series?

## El diseño de referencia

Los contadores mantienen sus conexiones MQTT en un Broker IoT EMQX con las 2 instancias por defecto. Las lecturas entran por un Servidor de aplicaciones Go (2 instancias), que publica cada una en un Stream Apache Kafka con 192 particiones. Seis Workers Celery (concurrencia 32 por instancia, tiempo de servicio 2 ms) consumen el stream y escriben en una Base de datos de series temporales InfluxDB con 3 instancias de 4000 puntos por segundo cada una. Cuesta unos 1405 $ al mes: 552 $ la base de datos, 420 $ los workers, 153 $ Kafka y 140 $ cada uno el broker y los Servidores de aplicaciones. Un día normal tiene un p95 de unos 56 ms desde la lectura hasta la base de datos, y ningún error.

El stream hace el trabajo en «La flota se reconecta tras un corte de red». El Servidor de aplicaciones solo publica, así que responde rápido haga lo que haga la base de datos. Los workers sacan lecturas del stream tan rápido como les dejan sus huecos, y ese ritmo queda por debajo de lo que escriben los tres nodos, así que no se rechaza nada: las lecturas de más esperan en Kafka, y los workers se ponen al día unos segundos después de que acabe la ráfaga. Mientras esperan, la frescura empeora. Esta prueba permite un p95 de 10 s, y el diseño consigue unos 4,8 s, con el peor segundo en unos 5,2 s y ningún error.

«Una actualización de firmware multiplica las series por 10» es donde importa el presupuesto de series. Con 3 instancias, cada nodo lleva 2 500 000 / 3 ≈ 833 000 series durante la explosión, todavía por debajo de su límite de 1 000 000. Las escrituras mantienen su velocidad normal, y el p95 se queda en unos 56 ms sin errores. Planificar las series por nodo con margen para una sorpresa de diez veces es lo que decide aquí el número de instancias, más que los puntos por segundo.

La búsqueda del reto de coste no encontró ningún diseño por debajo del 80 % del coste de la referencia (lo mejor que alcanzó fue un 85 %), así que este problema no tiene reto de coste.

## Por qué falla el primer intento

Un primer intento habitual tiene el mismo Broker IoT y los mismos Servidores de aplicaciones, con el Servidor de aplicaciones escribiendo directamente en un nodo InfluxDB de 4000 puntos por segundo: sin stream y sin workers. Cuesta unos 464 $ y supera «Un día normal» con un p95 de unos 52 ms; la base de datos va ocupada a un 75 %.

Cuando la flota se reconecta, llegan 10 000 lecturas por segundo a un nodo que escribe 4000. Su cola se llena en un momento y rechaza el resto: fallan unas dos terceras partes de las lecturas durante la ráfaga, un 44,2 % en toda la prueba. Añadir nodos a la base de datos sin un stream tampoco lo arregla. Con 3 nodos y sin stream, los Servidores de aplicaciones esperan ellos mismos cada escritura, sus huecos se llenan y sigue fallando un 34,2 % de las lecturas.

La actualización de firmware es peor para un solo nodo: 2 500 000 series en un nodo con un límite de 1 000 000 superan el doble del límite, así que rechaza todas las escrituras durante 20 segundos. Eso es un 32,7 % de errores en la prueba.

La idea es esta: una ráfaga que se puede esperar va en un stream, no en la cola de la base de datos. Un stream guarda las lecturas y los workers las entregan al ritmo que la base de datos aguanta, así que una ráfaga cuesta unos segundos de frescura en lugar de lecturas perdidas. Y dimensiona una base de series temporales por sus series además de por sus puntos: cuenta las series por nodo y deja margen para la etiqueta que alguien añadirá por error.

## Lo que el simulador deja fuera

El propio corte de red no se simula: los contadores siguen conectados todo el tiempo, y la prueba solo reproduce la subida de las lecturas guardadas como 4 veces el ritmo normal durante 20 segundos. La tormenta de reconexiones real en el broker (contadores que se caen y se reconectan a la vez) no forma parte de este problema. Aquí una lectura entra por una tarjeta de Servidor de aplicaciones que representa el puente del broker, porque el simulador empieza cada petición en un servicio; el ritmo de mensajes y la QoS del broker no afectan a las lecturas. Cada lectura es un punto; los contadores reales suelen agrupar muchos puntos en una escritura. Los puntos por segundo y el límite de series son lo que escribes en la tarjeta, no un tamaño de nodo medido, por eso el enunciado los fija. La retención, la reducción de resolución, la compresión y el tamaño en disco no se simulan, ni tampoco el precio de guardar los puntos.

## Cosas que probar

- Pon la base de datos a 2 instancias en la referencia y ejecuta la actualización de firmware. Cada nodo lleva 1 250 000 series, escribe cuatro veces más despacio y el stream se llena: el p95 sube a unos 5,9 s.
- Quita el stream y dale a la base de datos 3 instancias. Ejecuta la ráfaga de reconexión y mira los Servidores de aplicaciones, no la base de datos.
- Dale menos huecos a los workers y ejecuta la ráfaga de reconexión. Compara cuánto tarda en vaciarse el stream con el límite de 10 s.
