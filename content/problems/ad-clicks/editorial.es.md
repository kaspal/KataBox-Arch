# Solución comentada

## Adónde va la carga

Con 10 000 peticiones/s, unas 9900 son clics y unas 100 son lecturas de los totales de un anuncio. Cada clic es un mensaje en el stream y un incremento de un contador.

Kafka (Amazon MSK) se cobra por broker, no por mensaje: un broker cuesta 153 $ al mes, pasen los clics que pasen, y las particiones son gratis. La mayor parte de la factura son las instancias que hacen el trabajo.

Cada partición entrega un clic cada vez, así que las particiones limitan cuántos clics se cuentan a la vez, y el sitio donde vive el contador decide cuánto tarda cada clic. Un incremento en Redis lleva una fracción de milisegundo; una escritura en PostgreSQL, unos 5 ms, más la espera por un hueco de la base de datos. En las ejecuciones del juez, un clic ocupa un hueco unos 3,6 ms con los contadores en Redis y unos 10 ms con PostgreSQL. Con 128 particiones, eso da un techo de unos 35 000 clics por segundo con Redis y unos 12 800 con PostgreSQL.

## El diseño de referencia

Un Load Balancer (AWS ALB) envía los servidores de anuncios a 4 instancias Go de la Click collector. La collector publica cada clic en un stream Kafka con 128 particiones. Lo consumen 16 instancias Celery de Aggregators, con 1 ms de trabajo por clic; con la concurrencia por defecto de Celery, 8, son 128 huecos, uno por partición. Incrementan el contador del anuncio en una Cache Redis con un TTL de 86 400 s, y la collector lee los totales de ese mismo Redis. Cuesta unos 1614 $ al mes, de los que los Aggregators son 1120 $ y Kafka 153 $. En un día normal, los clics se cuentan con un p95 de unos 20 ms, y los Aggregators están ocupados en torno al 28 %.

«Un anuncio se hace viral» duplica el tráfico durante 20 segundos, a partir del segundo 20, y concentra los clics en unos pocos anuncios (Zipf 1,8), con un límite de p95 de 10 s. Unos 19 800 clics por segundo quedan muy por debajo del techo, así que el p95 se queda en unos 20 ms. «Los eventos atrasados llegan de golpe» multiplica el tráfico por 4 durante 10 segundos, a partir del segundo 20, con el límite de 30 s. Unos 39 600 clics por segundo superan un poco lo que cuentan 128 particiones, así que se acumula un atasco de unos 40 000 clics que luego se vacía en un par de segundos: el p95 ronda los 1,2 s. Kafka no cobra los mensajes extra, así que la factura se queda en unos 1614 $ en las dos.

## Por qué falla el primer intento

Un primer intento habitual es el mismo pipeline con los contadores en PostgreSQL: cada clic es un incremento en la base de datos, y los totales también se leen de ella. Cuesta unos 1928 $ al mes y supera «Un día normal» con un p95 de unos 31 ms, pero PostgreSQL ya está ocupado en torno al 65 % y los Aggregators en torno al 78 %: sus huecos pasan la mayor parte del tiempo esperando a la base de datos.

El doble de clics de la prueba viral es más de lo que PostgreSQL puede escribir y más de los 12 800 por segundo que los Aggregators pueden hacerle llegar. Los Aggregators se llenan esperando, las lecturas de totales hacen cola detrás de las escrituras y se descarta cerca del 5 % de las peticiones, con un p95 de unos 8 s. En la prueba de eventos atrasados se descarta cerca del 19 %. Más Aggregators solo esperarían a la misma base de datos.

## Lo que el simulador deja fuera

Los agregadores reales preagregan: suman en memoria los clics de cada anuncio y escriben el total de cada uno una vez por ventana, de un segundo o de un minuto, lo que requiere muchas menos escrituras. Los sistemas reales también cuentan por la hora del evento, el minuto en que ocurrió el clic y no cuándo llegó, y deciden cuánto esperar a los clics atrasados. Nada de eso se simula, ni tampoco el conteo exactamente una vez: un clic reintentado podría contarse dos veces. Los contadores que solo viven en la memoria de Redis pueden perderse en un reinicio; un sistema real guarda instantáneas en almacenamiento duradero. El simulador tampoco modela un anuncio muy popular que sobrecarga la única partición a la que van sus clics: la carga de la prueba viral es el tráfico extra.

## Para probar

- Baja a 64 las particiones de la referencia y ejecuta «Los eventos atrasados llegan de golpe». Observa cómo crece el atasco y sube el p95.
- Deja 128 particiones pero baja los Aggregators a 8 instancias (64 huecos). Mira cuántas particiones se quedan paradas.
- En el primer intento, duplica los Aggregators y ejecuta «Un anuncio se hace viral». Comprueba si más huecos ayudan cuando el límite es la base de datos.
