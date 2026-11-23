# Solución comentada

## Adónde va la carga

Con 15 000 peticiones/s, unas 14 700 son envíos de ubicación y unas 300 son peticiones de viaje. Es un problema de escrituras. Una caché no ayuda con los envíos, porque cada uno sustituye al anterior y nadie lo lee dos veces. Lo que ayuda es repartir las escrituras entre muchas máquinas independientes. Un NoSQL Store lo hace por clave: la clave de cada conductor cae siempre en la misma partición, así que la carga se divide entre particiones mientras ningún conductor acapare demasiado. Cassandra trabaja aquí con 32 peticiones a la vez por partición, a unos 3 ms cada una, así que cada partición recibe unas 10 000 escrituras por segundo.

## El diseño de referencia

Un Network Load Balancer reparte el tráfico entre 8 App Servers Go. Los envíos se escriben en Cassandra con 16 particiones, unas 160 000 escrituras por segundo de capacidad. Las peticiones de viaje leen una Cache Redis de 2 GB que guarda el índice de conductores cercanos.

Cuesta unos 4621 $ al mes, la mayor parte por los nodos de Cassandra: 16 particiones a 250 $ por nodo son 4000 $. El p95 ronda los 23 ms en un día normal. El aumento en toda la ciudad, que triplica el tráfico durante 25 segundos, deja cada partición muy por debajo de su límite. En el estadio, las claves de los conductores se concentran mucho (Zipf 1,3), así que una partición recibe mucho más de lo que le toca. Con 16 particiones, la caliente se mantiene bajo su límite y los errores quedan por debajo del 0,1 %.

## Por qué falla el primer intento

Un primer intento habitual usa los mismos App Servers, pero escribe cada envío en una sola PostgreSQL principal. PostgreSQL tiene 100 conexiones a unos 5 ms por escritura pequeña, así que su techo ronda las 20 000 escrituras por segundo. Un día normal cabe, con un p95 cercano a 31 ms. El aumento envía unas 44 000 escrituras por segundo, las conexiones se agotan, los huecos de los App Servers se acumulan esperándolas y falla alrededor del 44 % de las peticiones.

## Para probar

- Baja Cassandra a 4 particiones y ejecuta la prueba visible. Después busca la partición caliente.
- Reduce los App Servers a la mitad y comprueba si el aumento sigue pasando.
- Cambia el almacén a DynamoDB, que cobra cada escritura en vez de cada partición, y compara el coste mensual. ¿Qué mueve la factura, las particiones o las peticiones?
