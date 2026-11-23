# Solución comentada

## Adónde va la carga

Con 2000 rps, 1900 peticiones por segundo son consultas y 100 son canjes. Las consultas se reparten entre 5000 códigos, pero unos pocos códigos populares se llevan la mayoría, y los datos de un código son diminutos: los 5000 caben de sobra en una caché pequeña. Los canjes son pocos, pero 30 por segundo caen en el mismo código. Cuando la prueba de la influencer multiplica el tráfico por 6, son 12 000 rps: 11 400 consultas y 600 canjes por segundo, 180 de ellos en ese único código.

Así que la carga se divide en tres problemas distintos. Las consultas son muchas, repetidas y de solo lectura. Los canjes del código popular compiten por la misma cuenta. Y todo lo que escribe pasa por una única base de datos primaria, que una prueba oculta mata.

## El diseño de referencia

Un AWS ALB reparte a los clientes entre 2 Go App Servers. Las consultas pasan en cache-aside por una Redis Cache con 0,5 GB de memoria y un TTL de 60 s. Los datos viven en PostgreSQL: una primaria con «Escrituras condicionales (comprobar y escribir en un paso)» activado y un Tiempo de failover de 10 s, conectada con una arista Replicate a una segunda PostgreSQL con los mismos ajustes. Cuesta unos 874 $ al mes: 350 $ cada PostgreSQL, 140 $ los App Servers, 25 $ el ALB y 9 $ Redis. En un día normal el p95 es de unos 22,3 ms, sin errores.

Con escrituras condicionales, el canje es una sola sentencia: `UPDATE … SET uses_left = uses_left - 1 WHERE code = ? AND uses_left > 0`. La base de datos comprueba la cuenta y resta uno en el mismo paso, así que cuando dos canjes compiten por el último uso, uno gana y al otro se le dice «este código se ha agotado».

«Una influencer comparte un código» multiplica el tráfico por 6 a partir del segundo 15, durante 30 segundos, y no admite ningún conflicto. La caché absorbe las consultas de más, los App Servers dan abasto y la escritura condicional mantiene el código popular dentro de su límite: el p95 es de unos 21,8 ms, sin errores y con 0 conflictos.

«Falla la primaria de la base de datos» mata la primaria PostgreSQL en el segundo 20, y admite un 2 % de errores y un 98 % de disponibilidad. Durante unos 10 segundos, hasta que se promueve la réplica, falla cerca del 5 % de las peticiones de cada segundo. Después la réplica es la nueva primaria y todo vuelve a funcionar. La prueba termina con cerca de un 0,89 % de errores y un p95 de unos 21,4 ms.

La caché puede ir un poco desfasada. En un día normal, cerca del 27,6 % de las consultas salen de una copia en caché anterior al último canje del código, y cerca del 58 % durante el pico de la influencer, así que los usos restantes que se ven al pagar pueden ir un poco por detrás. No pasa nada: la consulta es solo una pista, y lo que hace cumplir el límite es el canje condicional.

Este problema no tiene reto de coste. Ningún diseño más barato superó todas las pruebas: la referencia ya es ajustada, y sus dos tarjetas PostgreSQL son la mayor parte de la factura por una buena razón.

## Por qué falla el primer intento

Un primer intento habitual es el mismo edge y 2 Go App Servers, con una sola PostgreSQL, sin caché y sin escrituras condicionales. Cuesta unos 515 $ al mes y es Aceptado en «Un día normal», con un p95 de unos 29,9 ms. Pero ya canjea de más: en esa ejecución hay 594 conflictos; esa prueba simplemente no los evalúa.

En la prueba de la influencer se rompe de dos formas a la vez. Cada consulta va a la base de datos, y el App Server conserva su hueco mientras espera. En un día normal los 2 App Servers están ocupados en un 22 %; con 6 veces el tráfico harían falta cerca de un 130 %. Se llenan, falla cerca del 20,6 % de las peticiones y el p95 sube a unos 140 ms. Mientras tanto, los canjes de leer y después escribir compiten por el código popular: 3692 conflictos, cada uno un uso que la tienda nunca quiso regalar.

En la prueba del failover falla cerca del 66,8 % de las peticiones. Sin réplica, nada toma el relevo cuando muere la primaria, y todas las peticiones que necesitan la base de datos fallan hasta el final de la ejecución.

La idea es esta: tres costumbres distintas. Cachea las consultas que casi solo se leen. Haz cumplir un límite con una escritura condicional, no leyendo y después escribiendo. Ten una réplica a la que pasar el relevo. Cada una arregla un fallo distinto, y ninguna sustituye a otra.

## Lo que el simulador deja fuera

Un UPDATE condicional real sobre una fila toma un bloqueo de fila, así que los canjes del código popular hacen cola uno detrás de otro; aquí esa serialización no añade tiempo. Tampoco se simulan los límites por usuario («un uso por cliente»), las fechas de caducidad de los códigos ni las comprobaciones antifraude.

## Para probar

- Desactiva «Escrituras condicionales (comprobar y escribir en un paso)» en la referencia y ejecuta la prueba de la influencer. Cuenta los conflictos.
- Quita la Redis Cache de la referencia y vuelve a ejecutar la prueba de la influencer. Mira lo ocupados que quedan los App Servers.
- Pon el TTL de la caché en 5 s y ejecuta «Un día normal». Compara las lecturas desfasadas y el p95 con un TTL de 60 s.
