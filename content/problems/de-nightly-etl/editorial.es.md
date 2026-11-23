# Solución comentada

## Adónde va la carga

Mide la base de datos en conexiones ocupadas. Una tarjeta PostgreSQL tiene una concurrencia de 100: trabaja en 100 consultas a la vez, y el resto espera en la fila. Leer un pedido lleva unos 5 ms. Una página de 576 KB lleva 5 × (1 + 576 / 64) = 50 ms de mediana, y unos 64 ms de media cuando se cuenta la cola lenta.

La app manda 1400 consultas por segundo de unos 6 ms cada una: más o menos 9 conexiones ocupadas. La exportación manda 600 páginas por segundo de unos 64 ms cada una: más o menos 38. Juntas no llegan a la mitad de una base de datos, y la prueba visible pasa con una sola principal.

«La exportación lleva el triple de datos» mantiene la misma ventana nocturna, así que la exportación lee páginas tres veces más rápido: 1800 por segundo, más o menos 115 conexiones ocupadas. Eso es más de lo que tiene una base de datos. Cuando todas las conexiones están ocupadas, cada consulta nueva espera en la misma fila, y las lecturas rápidas de la app esperan detrás de páginas de la exportación que tardan 50 ms o más cada una.

La exportación solo lee, y puede vivir con datos de hace unos milisegundos. Esas lecturas no tienen por qué caer en la tarjeta que tiene que aceptar todas las escrituras.

## El diseño de referencia

Un AWS ALB reparte el tráfico de la app entre 2 Servidores de aplicaciones Go, conectados por Consulta a una Base de datos SQL PostgreSQL: la principal. Un Planificador Airflow publica las páginas de la exportación en una Cola RabbitMQ. Un Worker Celery con 4 instancias, una concurrencia por instancia de 64 y un tiempo de servicio de 2 ms las consume y está conectado por Consulta a una segunda Base de datos SQL PostgreSQL, que la principal alimenta con Replicar. «Leer de» se queda en «Principal y réplicas». Cuesta unos 1235 $ al mes: 350 $ cada base de datos, 280 $ los Workers, 140 $ los Servidores de aplicaciones, 70 $ la Cola, 25 $ el ALB y 20 $ el Planificador.

Las dos bases de datos forman un clúster. Las escrituras van a la principal; las lecturas se reparten entre la principal y la réplica. En «Una noche con la exportación en marcha» la principal está ocupada al 24 % y la réplica al 23 %, donde una principal sola estaría al 47 %. El p95 es de unos 104 ms, que es sobre todo el tiempo de lectura de las propias páginas de la exportación, sin errores. Un 0,09 % de las lecturas son obsoletas: una lectura de la app que llegó a la réplica dentro de su retraso de replicación de 50 ms después de actualizarse el pedido.

En «La exportación lleva el triple de datos» cada base de datos se lleva más o menos la mitad de las unas 115 conexiones ocupadas de la exportación, más su parte de la app. Ninguna se queda sin conexiones, y el p95 es de unos 132 ms sin errores.

«Una réplica cae durante la exportación» mata la réplica en el segundo 20, durante 40 segundos, en una ejecución de 90 segundos. Esta prueba admite un 3 % de errores y un 97 % de disponibilidad. Durante unos dos segundos, hasta que las comprobaciones de salud se dan cuenta, las lecturas siguen yendo a la réplica caída y fallan: un 0,67 % de las peticiones de toda la ejecución. Después la principal se lleva todas las lecturas. Al ritmo normal de la exportación, una base de datos puede con eso, como muestra el primer intento. Las páginas de la exportación que pilló el fallo se vuelven a intentar desde la Cola, lo que les cuesta unos segundos. El p95 es de unos 106 ms.

Este problema no tiene reto de coste. La búsqueda de un diseño más barato que supere todas las pruebas no bajó del 83 % del coste de la referencia, y no puede quitar ninguna de las dos bases de datos: una sola tarjeta no puede con la exportación triplicada y la app a la vez.

## Por qué falla el primer intento

Un primer intento habitual es el mismo diseño con una sola Base de datos SQL PostgreSQL: los Servidores de aplicaciones y los Workers de la exportación leen los dos de la principal. Cuesta unos 885 $ y supera «Una noche con la exportación en marcha» con un p95 de unos 104 ms y la principal ocupada al 47 %.

En «La exportación lleva el triple de datos» la principal se queda sin conexiones en los primeros segundos. Las páginas de la exportación se acumulan en la Cola más deprisa de lo que la base de datos puede leerlas, así que su espera crece cada segundo, y el p95 llega a unos 11,8 segundos. La app sufre con ella: sus lecturas y actualizaciones esperan en la misma fila que las páginas de la exportación, los huecos de los Servidores de aplicaciones se llenan de peticiones que esperan a la base de datos, y a partir del segundo 3 más o menos falla alrededor del 10 % de todas las peticiones. En toda la prueba son un 9,87 % de errores, y cada uno es la petición de un cliente de la tienda.

Bajar los Workers de la exportación para que ocupen menos conexiones no la salva: entonces la exportación no puede leer 1800 páginas por segundo, y las páginas que esperan en la Cola también cuentan en el p95.

La idea es esta: una lectura de análisis y la lectura de un cliente compiten por las mismas conexiones, y gana la pesada. Dale al trabajo pesado y de solo lectura su propia copia de los datos, para que la principal guarde sus conexiones para la app y para las escrituras.

## Lo que el simulador deja fuera

En un sistema real apuntas la exportación a la dirección propia de la réplica, así que la principal nunca la ve. El simulador reparte todas las lecturas, las de la app y las de la exportación, entre todos los miembros sanos del clúster, así que aquí la réplica se queda con la mitad de la carga de lectura en vez de llevarse ella sola la exportación. El retraso de replicación es fijo, de 50 ms: no se simula una réplica que se queda atrás mientras reproduce mucha carga de escritura, ni consultas en una réplica canceladas porque chocan con esa reproducción. Tampoco se simulan los discos, la caché de búferes que un escaneo grande vacía, el vacuum ni el registro de escritura anticipada (WAL). Una exportación real dura horas; las pruebas simulan entre un minuto y un minuto y medio a su ritmo de trabajo, y el intervalo del Planificador no cambia la carga. Escribir los ficheros de análisis no forma parte del problema. Subir la concurrencia por instancia de una base de datos aquí no cuesta nada; una base de datos real tiene un número fijo de núcleos y discos, así que eso no le daría más capacidad.

## Para probar

- Pon «Leer de» en «Solo la principal» en la principal de la referencia y ejecuta la prueba del triple. Compárala con el primer intento.
- Deja el Worker Celery del primer intento en 1 instancia y ejecuta la prueba visible. Fíjate en la espera de las páginas de la exportación.
- Ejecuta «Una réplica cae durante la exportación» con la referencia y mira los segundos justo después de matarla.
