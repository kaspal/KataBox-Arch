# Solución comentada

## Adónde va la carga

Cada petición es una orden: 10 000 por segundo. Un matcher dedica 0,5 ms a cada una y la escribe en el libro de órdenes Redis. Desde que el stream entrega una orden hasta que el matcher termina pasan unos 3 ms: el salto, el casamiento y el viaje de ida y vuelta a Redis.

Un stream entrega un mensaje por partición cada vez, así que una partición casa unas 330 órdenes por segundo. 48 particiones casan unas 16 000 por segundo, y 128 unas 42 700. Una instancia Go de la pasarela atiende 64 peticiones a 5 ms cada una, unas 12 800 por segundo. En la apertura llegan cuatro veces más órdenes, 40 000 por segundo: eso exige al menos 4 instancias en la pasarela y unas 120 particiones.

## El diseño de referencia

La Order gateway, con 6 instancias Go detrás de un Network Load Balancer, publica cada orden en un stream Kafka, el Order log, con 128 particiones. Lo consumen dos tarjetas de matchers Celery, Matchers A y Matchers B, cada una con 12 instancias y la concurrencia por defecto de Celery, 8: 96 huecos por tarjeta, 192 en total. Las dos escriben en un libro de órdenes Redis con un TTL de 86 400 segundos. Cuesta unos 7447 $ al mes, sobre todo por el stream: unos 5286 $, porque Kafka se cobra por mensaje y 10 000 órdenes por segundo son unos 26 000 millones de mensajes al mes. En una sesión normal el p95 es de unos 19 ms, con la pasarela y los matchers ocupados en torno al 16 %.

«Abre el mercado» multiplica el tráfico por cuatro durante 10 segundos, a partir del segundo 20, y admite un p95 de 1 s y un p99 de 3 s. 40 000 órdenes por segundo caben bajo el techo de 128 particiones, y 6 instancias en la pasarela aguantan unas 76 800 por segundo, así que no se forma atasco: el p95 se queda en unos 19 ms.

«Se cae un matcher» mata la primera tarjeta Worker, Matchers A, en el segundo 20 y para siempre, en una ejecución de 90 segundos, y admite un p95 de 100 ms y un p99 de 1 s. Matchers B recibe enseguida todas las órdenes nuevas; sus 96 huecos casan unas 32 000 por segundo. El puñado de órdenes que tenía A siguen asignadas a ella hasta que vence el tiempo de sesión del stream (el valor por defecto de Kafka, 45 segundos), y luego pasan a B. Son unas pocas decenas entre unas 900 000, muchas menos que 1 de cada 100, así que no mueven ni el p95 (unos 19 ms) ni el p99 (unos 26 ms).

## Por qué falla el primer intento

Un primer intento habitual usa 3 instancias en la pasarela, 48 particiones y una sola tarjeta de matchers con 8 instancias: 64 huecos, por unos 6117 $ al mes. Supera «Una sesión normal» con el mismo p95 de unos 19 ms; los matchers están ocupados en torno al 47 %.

En la apertura se queda corto por dos lados. 3 instancias en la pasarela aguantan unas 38 400 órdenes por segundo, justo por debajo de las 40 000 que llegan, y 48 particiones solo casan unas 16 000. El atasco crece durante los 10 segundos: en torno al 16 % de las órdenes fallan y el p95 llega a unos 6,4 s.

Cuando cae su única tarjeta de matchers, a partir del segundo 20 no se casa nada: en torno al 67 % de las órdenes cuentan como errores y el p95 llega a unos 66 segundos. No se pierde ninguna orden; el stream las guarda todas.

## Lo que el simulador deja fuera

Las bolsas reales casan cada valor en un solo hilo y en memoria, en estricto orden de precio y de llegada, normalmente sin un bróker en medio, y sus latencias se miden en microsegundos. Los milisegundos del simulador muestran la forma del diseño, no las cifras absolutas. Tampoco se simulan la equidad ni el orden dentro de cada valor. El tiempo de sesión es un compromiso real (demasiado largo y las órdenes de un consumidor caído esperan; demasiado corto y un consumidor que solo va lento un momento queda expulsado del grupo), pero aquí retrasa demasiado pocas órdenes para notarse en los percentiles.

## Para probar

- Junta las dos tarjetas de matchers en una sola de 24 instancias y vuelve a ejecutar la prueba del matcher. Los mismos huecos, el mismo coste, otro resultado.
- Junta las dos tarjetas de matchers en una sola de 24 instancias y ejecuta «Se cae un matcher». Los mismos huecos, el mismo coste, un resultado muy distinto.
- Mantén 128 particiones pero baja la pasarela a 3 instancias. Mira qué parte de la apertura sigue fallando.
