# Solución comentada

## Adónde va la carga

Con 4 000 mensajes/s, cada mensaje pasa dos veces por cómputo: una en el gateway de quien lo envía, que lo publica, y otra en el gateway del destinatario, que lo entrega. Entre ambos está el broker. Un Pub/Sub entrega una copia de cada mensaje a cada gateway suscrito, y las peticiones de pub/sub crecen con el tráfico: son la mayor parte de la factura. El trabajo por mensaje es pequeño. Lo difícil es que la entrega siga funcionando cuando falla un gateway y cuando la tarde se anima.

## El diseño de referencia

Un Network Load Balancer que admite WebSockets reparte las conexiones entre dos tarjetas de gateway, Gateway A y Gateway B, cada una con 2 instancias Go. Ambas publican en un Pub/Sub Redis, el «bus de mensajes», y ambas consumen de él, así que el gateway que tenga al destinatario puede entregar.

Cuesta unos 5 590 $ al mes y su p95 ronda los 28 ms en una tarde normal. Cuando Gateway A se cae, fallan los mensajes que estaba atendiendo, y durante un momento el Load Balancer sigue enviándole tráfico. Las comprobaciones de salud lo detectan en 2 a 4 segundos y las conexiones pasan a Gateway B. Falla alrededor del 1,7 % de los mensajes y algunas entregas esperan cerca de un segundo; después, Gateway B lo lleva todo. El diseño también aguanta el pico de la noche.

## Por qué falla el primer intento

Un primer intento habitual es una sola tarjeta de gateway con 2 instancias que publica en el bus y consume de él. En una tarde normal va bien, con un p95 cercano a 26 ms, incluso algo más rápido que la referencia.

Las pruebas ocultas encuentran los huecos. «Se cae un gateway» mata la tarjeta de gateway a los 20 segundos. Admite hasta un 2 % de errores y un p95 de 2 segundos, porque se espera un corte breve. Con una sola tarjeta no hay adónde mover las conexiones, y falla alrededor del 67 % de los mensajes. «Pico de la noche» triplica el tráfico durante 25 segundos. Dos instancias no dan abasto: sus hilos se llenan, los mensajes hacen cola y luego fallan, y se pierde alrededor del 5 %.

## Para probar

- Quita Gateway B y ejecuta «Una tarde normal». Compara el coste y el p95, y luego pregúntate qué te da la segunda tarjeta.
- Añade una tercera tarjeta de gateway y vuelve a matar una: compara los errores mientras los health checks se ponen al día.
- Baja cada gateway a 1 instancia y mira cuánto se acerca una tarde normal al límite.
