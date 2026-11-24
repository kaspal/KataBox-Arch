# Solución comentada

## Adónde va la carga

La mayor parte de la carga son las conexiones abiertas, no las cargas de página. 5 000 000 de aficionados reciben cada uno una actualización cada 50 segundos de media: 5 000 000 ÷ 50 = 100 000 mensajes por segundo. Cada conexión abierta ocupa además unos 20 KB de memoria en su gateway, así que 5 000 000 de aficionados necesitan unos 100 GB en total. Una instancia de gateway con 32 GB podría alojar 1,6 millones de conexiones por memoria, pero su propio límite es 1 000 000.

Las pruebas ocultas aprietan por los dos lados. Un gol al final multiplica por diez las actualizaciones durante 10 segundos: 1 000 000 de mensajes por segundo, junto con diez veces las cargas de página. Un gateway caído suelta de golpe a todos los aficionados que tenía, y todos quieren reconectar. El resto de gateways necesita sitio para ellos, y necesita aceptarlos a un ritmo que pueda llevar.

Las páginas de partido son la parte fácil: 2000 rps repartidas entre 2000 partidos, con unos pocos partidos grandes llevándose la mayoría de las visitas, así que una caché de vida corta absorbe la mayoría de las lecturas.

## El diseño de referencia

El Cliente, con la Variación al reconectar en 20 s, se conecta a través de un AWS ALB a 4 tarjetas Gateway de WebSocket Socket.IO. Cada tarjeta tiene 2 instancias con Conexiones máximas 1 000 000, Memoria 32 GB, Mensajes por segundo 200 000 y Conexiones nuevas por segundo 20 000. Eso da 8 000 000 de huecos de conexión para 5 000 000 de aficionados (llenos al 60 %) y 1 600 000 mensajes por segundo. Las páginas de partido van a 4 servidores de aplicaciones Go, una caché Redis con un TTL de 2 s y PostgreSQL. Cuesta 1131 $ al mes: PostgreSQL 260 $, los servidores de aplicaciones 280 $, 140 $ por cada tarjeta de gateway, el ALB 25 $ y Redis 6 $.

«Una jornada con mucho partido» se supera sin errores, y todos los aficionados siguen conectados.

«Un gol en el último minuto» envía diez veces los mensajes por aficionado (1 000 000 por segundo) y diez veces las cargas de página, a partir del segundo 20, durante 10 segundos. Se supera sin errores: los mensajes se quedan por debajo de los 1 600 000 por segundo que pueden enviar los gateways, y los servidores de aplicaciones pasan de estar ocupados en torno al 7 % a en torno al 70 %.

«Cae una pasarela en el descanso» mata para siempre la primera tarjeta de gateway en el segundo 20, en una ejecución de 120 segundos, y baja el objetivo de conectados al 70 %. Se supera sin errores. Los 1 250 000 aficionados de esa tarjeta vuelven en unos 20 segundos a las 6 instancias restantes, que tienen sitio para 6 000 000 y aceptan 120 000 conexiones nuevas por segundo. La proporción de conectados baja a un 75 % aproximadamente en el momento de la caída y se recupera.

Lo que hace que funcione es la variación. Sin ella, la misma referencia no supera la prueba: se rechazan alrededor del 98 % de los intentos de conexión durante más de 30 segundos. El margen por sí solo no sirve cuando 1 250 000 clientes reconectan en los mismos 100 ms. Con una espera aleatoria de 0 a 20 segundos, llegan a unos 62 000 por segundo, por debajo de los 120 000 de los supervivientes.

Aquí no hay reto de coste: nada más barato superó todas las pruebas. Los gateways son baratos; la base de datos es la partida más grande.

## Por qué falla el primer intento

Un primer intento habitual no tiene variación en el Cliente y usa 3 tarjetas de gateway Socket.IO de 2 instancias, con los mismos ajustes salvo Mensajes por segundo 100 000, y 2 servidores de aplicaciones Go. Son 6 000 000 de huecos para 5 000 000 de aficionados (un 20 % de margen) y 600 000 mensajes por segundo. Cuesta 851 $ al mes y supera «Una jornada con mucho partido».

«Un gol en el último minuto» falla con un 10,7 % de errores y un p95 de 6,2 s. Una ráfaga de 1 000 000 de mensajes por segundo se encuentra con 600 000 por segundo de capacidad, y el atasco que se forma tarda unos 10 segundos en vaciarse cuando acaba la ráfaga.

«Cae una pasarela en el descanso» falla de lleno: se rechaza el 99,7 % de los intentos de conexión. 2 000 000 de aficionados pierden su gateway a la vez y todos reintentan en el mismo instante, segundo tras segundo, así que los supervivientes rechazan casi todos los intentos. Y aunque llegaran perfectamente repartidos, en los 4 000 000 de huecos que quedan no caben 5 000 000 de aficionados, así que 1 000 000 no volverían nunca.

La idea es esta: dimensiona los gateways de conexiones para N−1, de modo que al perder uno sigan cabiendo todos; guarda margen de mensajes para las ráfagas; y haz que los clientes reconecten con variación aleatoria (y esperas crecientes), para que un fallo se convierta en un flujo constante de reconexiones en vez de una tormenta.

## Lo que el simulador deja fuera

No hay salas por partido: cada aficionado recibe actualizaciones al ritmo medio de la plataforma, no solo las del partido que sigue. No se modelan las sesiones persistentes ni el estado de las conexiones, ni las redes móviles que cortan conexiones por su cuenta. Tampoco se simulan el coste de CPU de los handshakes TLS, los límites de conexiones del propio ALB ni el ancho de banda.

## Para probar

- Pon la Variación al reconectar del Cliente en 0 en la referencia y ejecuta «Cae una pasarela en el descanso». ¿Cuánto duran los intentos rechazados?
- Baja los Mensajes por segundo de cada tarjeta de gateway a 100 000 en la referencia y ejecuta «Un gol en el último minuto». ¿Vuelve el atasco?
- Quita la cuarta tarjeta de gateway de la referencia, mantén la variación y ejecuta la prueba del gateway. ¿Basta la variación cuando los supervivientes no tienen sitio?
