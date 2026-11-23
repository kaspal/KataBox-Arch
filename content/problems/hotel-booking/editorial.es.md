# Solución comentada

## Adónde va la carga

Con 5000 rps, 4900 peticiones por segundo son búsquedas y 100 son reservas. Las búsquedas se reparten entre 200 000 consultas distintas, pero no por igual: con este sesgo, la búsqueda más popular es casi el 8 % de todas, cerca de 400 por segundo. Un motor de búsqueda envía cada búsqueda a la partición que la contiene, así que esa búsqueda cae siempre en la misma partición, haya las que haya. Cuando se abren las reservas de vacaciones, el tráfico se triplica a 15 000 rps: 14 700 búsquedas por segundo.

Las reservas son pocas, pero el 20 % de ellas, 20 por segundo, son de las mismas 10 habitaciones. En la prueba de la última habitación las reservas se multiplican por 8, hasta 800 por segundo, y 160 por segundo compiten por esas 10 habitaciones: unas 16 por segundo para cada habitación.

Así que la carga se divide en dos problemas distintos. Las búsquedas son muchas, repetidas y de solo lectura, y las populares se amontonan en una partición. Las reservas de las habitaciones más buscadas compiten por la misma noche de la misma habitación.

## El diseño de referencia

Un AWS ALB reparte a los viajeros entre 4 Go App Servers. Las búsquedas pasan en cache-aside por una Redis Cache con 4 GB de memoria y un TTL de 300 s, delante de Elasticsearch con 4 shards. Las reservas van a PostgreSQL con «Escrituras condicionales (comprobar y escribir en un paso)» activado. Cuesta unos 1227 $ al mes: 500 $ Elasticsearch, 350 $ PostgreSQL, 280 $ los App Servers, 72 $ Redis y 25 $ el ALB. En «Una semana normal» el p95 es de unos 27,1 ms, sin errores, y el motor de búsqueda está ocupado en torno a un 14 %.

Con escrituras condicionales, la reserva es una sola sentencia que falla si esa noche de la habitación ya está ocupada: por ejemplo, una restricción única sobre (habitación, noche), o `UPDATE … SET booked = true WHERE room = ? AND night = ? AND NOT booked`. La base de datos comprueba y escribe en el mismo paso, así que cuando dos reservas compiten por la última habitación, una gana y a la otra se le dice «la acaba de reservar otra persona».

La caché es lo que mantiene tranquilo al motor de búsqueda. Las búsquedas populares se responden desde Redis y nunca llegan a Elasticsearch, así que ninguna partición se convierte en un punto caliente. El ajuste mostró por qué hace falta: sin caché, incluso 20 shards de Elasticsearch (2500 $ solo en búsqueda) seguían perdiendo cerca del 1 % de las búsquedas en una semana normal, porque la búsqueda más popular seguía yendo a una sola partición, y los 4 App Servers se llenaban esperando a la búsqueda.

«Se abren las reservas de vacaciones» triplica el tráfico a partir del segundo 15, durante 30 segundos. Las búsquedas de más son sobre todo las mismas populares, así que la caché las absorbe: el p95 queda en unos 26,1 ms, sin errores.

«Todos quieren la última habitación» multiplica las reservas por 8 y no admite ningún conflicto. En este problema se simulan todas las reservas, no una muestra. La escritura condicional deja ganar a una sola reserva por cada noche de habitación: el p95 queda en unos 31,8 ms, con 0 conflictos. Los que pierden reciben una respuesta, no un error: otra persona acaba de reservar la habitación.

La caché puede estar un poco desactualizada. Cerca del 22 % de las búsquedas salen de un resultado en caché anterior a la última reserva, así que una búsqueda puede mostrar una habitación que acaba de venderse. No pasa nada: la búsqueda es solo una pista, y lo que decide es la reserva.

Para el reto de coste, un diseño de prueba se queda en el 69 % del coste de la referencia, por debajo del objetivo del 70 %. Mantiene la misma forma y la escritura condicional, con menos App Servers, una caché mucho más pequeña y menos shards de búsqueda: cuando la caché se lleva las búsquedas populares, al motor de búsqueda le queda poco trabajo.

## Por qué falla el primer intento

Un primer intento habitual es el mismo diseño con las escrituras condicionales de PostgreSQL desactivadas: el App Server lee la disponibilidad y después inserta la reserva. Cuesta lo mismo, unos 1227 $ al mes, y supera «Una semana normal» y «Se abren las reservas de vacaciones» con los mismos resultados.

«Todos quieren la última habitación» lo rompe. 160 reservas por segundo compiten por 10 habitaciones, y muchas parejas leen «libre» antes de que ninguna escriba. Las dos salen bien: 1992 reservas dobles en una ejecución, cada una un viajero que llegará y encontrará la habitación ocupada. La prueba no admite ninguna, así que el diseño falla.

La idea es esta: dos hábitos distintos. Cachea las búsquedas populares, porque las particiones reparten muchas búsquedas pero nunca dividen una búsqueda popular. Y deja que la base de datos rechace la segunda reserva, con una restricción o una escritura condicional, en vez de comprobar primero y escribir después.

## Lo que el simulador deja fuera

No se simulan las réplicas de Elasticsearch: la propiedad replicas no cambia el resultado. Tampoco se simulan los calendarios de disponibilidad reales ni las reservas de varias noches; aquí una reserva es una sola clave. Quedan fuera las retenciones de pago y las políticas de sobreventa. Un servicio de bloqueos, un lock en Redis o etcd, también evitaría las reservas dobles, pero una restricción en la base de datos es más sencilla.

## Para probar

- Desactiva «Escrituras condicionales (comprobar y escribir en un paso)» en la referencia y ejecuta «Todos quieren la última habitación». Cuenta los conflictos.
- Quita la Redis Cache de la referencia y ejecuta «Una semana normal». Después sube Elasticsearch a 20 shards y vuelve a ejecutarla: ¿desaparece la tasa de errores?
- Pon el TTL de la caché en 30 s y ejecuta «Una semana normal». Compara las lecturas desactualizadas y lo ocupado que está el motor de búsqueda con un TTL de 300 s.
