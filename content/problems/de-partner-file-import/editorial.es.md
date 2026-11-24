# Editorial

## Dónde va la carga

A 50 peticiones por segundo, unas 39 son ficheros normales, 1 es un fichero gigante y 10 son consultas de estado. La subida es barata: el App Server escribe el fichero en S3, publica un mensaje y responde. El trabajo está en los Workers, que ocupan un hueco mientras analizan el fichero (100 ms), lo leen de S3 (50 ms) y escriben sus filas en PostgreSQL (unos 25 ms para 256 KB). Con los saltos de red y las colas lentas, un fichero normal ocupa un hueco unos 0,2 s, y uno gigante 25 veces eso, unos 5 s. Un día normal mantiene ocupados unos 39 × 0,2 + 1 × 5 ≈ 13 huecos.

Las pruebas ocultas cambian las dos mitades. «Un socio envía un mes de ficheros gigantes» multiplica los ficheros gigantes por 20, hasta 20 por segundo durante todo el minuto: unos 39 × 0,2 + 20 × 5 ≈ 108 huecos ocupados, ocho veces un día normal. «Se cae la primaria de la base de datos» mata la primaria de PostgreSQL a los 20 segundos de una ejecución de 2 minutos, y no vuelve. Todas las escrituras fallan hasta que se promueve una réplica, unos 30 segundos después, y cada fichero que llega en ese tiempo tiene que esperar.

## El diseño de referencia

El servidor del socio llama a 2 App Servers Go. Estos escriben cada fichero en Amazon S3, publican un mensaje en una Cola Amazon SQS y leen PostgreSQL para las consultas de estado. La cola tiene «Reintentos máximos» en 8: las esperas suman 1 + 2 + 4 + 8 + 16 + 32 + 32 + 32 = 127 segundos, mucho más que un failover. Una Cola de mensajes fallidos está conectada a la cola con Publicar. 16 Workers Celery con concurrencia 8 (128 huecos) consumen la cola, leen el fichero de S3 y escriben en PostgreSQL. PostgreSQL está conectada con Replicar a una segunda PostgreSQL, con el tiempo de failover por defecto de 30 s. Esto cuesta unos 2472 $ al mes: 1120 $ de Workers, 566 $ de S3, 126 $ de SQS, 140 $ de App Servers y 260 $ por cada PostgreSQL. Un día normal tiene un p95 de unos 489 ms, sin errores y sin ficheros perdidos.

Con los ficheros gigantes, los Workers están ocupados un 87 % y el p95 es de unos 7,2 s, sobre todo por el propio trabajo de los ficheros gigantes. El mes sale más caro a ese ritmo (2826 $), porque S3 y SQS cobran por petición.

«Se cae la primaria de la base de datos» permite un p95 de 45 s, un 2 % de errores y un 98 % de disponibilidad. Durante unos 2 segundos, hasta que los chequeos de salud lo notan, las lecturas de estado siguen yendo a la primaria caída y algunas fallan; después las sirve la réplica. Las escrituras fallan hasta que la réplica se promueve, unos 30 segundos después de la caída. Cada fichero que falla vuelve a la cola y se intenta de nuevo: los primeros en fallar, a los 20 s, pasan en su reintento de los 51 s, unos 31 s tarde. El p95 es de unos 31,9 s, con un 0,149 % de errores y ningún fichero perdido. La Cola de mensajes fallidos se queda vacía: quítala y vuelve a ejecutar la prueba, y tampoco se pierde nada.

Este problema no tiene reto de coste: la búsqueda encontró un diseño al 90 % del coste de la referencia, no por debajo del 80 % que necesita un reto.

## Por qué falla el primer intento

Un primer intento habitual es el mismo camino con 3 Workers Celery (24 huecos), los 3 reintentos por defecto de la cola, sin Cola de mensajes fallidos y con una sola PostgreSQL. Cuesta unos 1302 $ al mes y supera «Un día normal» con el mismo p95 de unos 489 ms; sus Workers están ocupados un 57 %.

Con los ficheros gigantes, 24 huecos se enfrentan a unos 108 huecos de trabajo. Los Workers están ocupados un 99 %, la cola crece durante todo el minuto y el p95 pasa del minuto.

Cuando cae la primaria, no hay nada que promover. Todas las consultas de estado fallan desde entonces, un 16,9 % de errores. Cada fichero falla su escritura, se reintenta tras 1, 2 y 4 segundos y después se pierde. Los Workers siguen ocupados un 93 % con escrituras que no pueden salir bien, la cola se atasca detrás de ellos, y al final se pierden 1794 ficheros y el p95 pasa del minuto. Ni siquiera con una réplica bastan 3 reintentos: 7 segundos de espera se acaban mucho antes de que termine un failover de 30 segundos, y se pierden unos 380 ficheros. Una Cola de mensajes fallidos tampoco lo salva: los mismos 380 ficheros esperan en la Cola de mensajes fallidos, y la prueba falla por ellos, porque un fichero en esa cola es un fichero sin cargar.

La idea es esta: una cola no pone el trabajo a salvo por sí sola. Los reintentos tienen que durar más que el fallo que esperas, la base de datos necesita una copia que pueda tomar el relevo, y los Workers necesitan sitio para los ficheros más grandes, no para el fichero medio.

## Lo que el simulador deja fuera

El juez cuenta los ficheros que acaban en la Cola de mensajes fallidos, pero no se simula reprocesarlos: en la vida real alguien tiene que encontrar cada uno y volver a enviarlo.

Aquí analizar y cargar son un solo paso del Worker: un cargador real leería el fichero en streaming, lo escribiría por lotes y haría cada lote idempotente, para que un reintento no inserte las mismas filas dos veces. No se simulan filas duplicadas, cargas parciales ni ficheros mal formados. Un fichero gigante se modela como 25 veces el trabajo en cada paso, no como un fichero de un tamaño concreto. Nunca se comprueba el contenido de las filas.

## Cosas que probar

- Deja 8 reintentos pero quita la réplica, y ejecuta «Se cae la primaria de la base de datos». Después baja los «Reintentos máximos» a 5 (31 s de espera) y luego a 4, con la réplica de vuelta.
- Baja los Workers a 12 y vuelve a ejecutar los ficheros gigantes. Después prueba un grupo de autoescalado.
- Pon el tiempo de failover de PostgreSQL en 10 s y mira cómo cambia el p95 del failover.
