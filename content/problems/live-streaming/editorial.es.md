# Solución comentada

## Adónde va la carga

Aquí hay dos cargas, y tienen formas distintas. El vídeo es el 95 % de las peticiones: 5000 × 0,95 = 4750 descargas de segmentos por segundo, todas lecturas, sobre solo 3000 segmentos en directo, y los canales grandes se llevan la mayoría (Zipf 1,2). Miles de espectadores piden el mismo segmento en los mismos pocos segundos. Mandar cada una de esas descargas al almacenamiento de origen lo satura en una tarde normal; una CDN que guarda cada segmento unos segundos convierte 4750 descargas por segundo en un pequeño flujo de fallos de caché.

El chat es un problema de difusión. Solo se envían 250 mensajes por segundo, pero 200 000 espectadores reciben cada uno 0,5 mensajes por segundo: 200 000 × 0,5 = 100 000 mensajes por segundo que entregar. Los gateways tienen que mantener las conexiones, y además tienen que enviar los mensajes.

Las pruebas ocultas aprietan cada lado. Una raid lo multiplica todo por cuatro durante 20 segundos: 19 000 descargas de segmentos por segundo, 1000 mensajes enviados y 400 000 mensajes por segundo que entregar. Un origen lento añade 400 ms a cada descarga que llega a él, más que todo el límite de p95 de 300 ms.

## El diseño de referencia

Los segmentos pasan por una CDN CloudFront con un TTL de 4 s hasta un bucket de Amazon S3. Los espectadores del chat se conectan desde el Cliente, con la Variación al reconectar en 10 s, a través de un AWS ALB a 2 tarjetas Gateway de WebSocket Socket.IO. Cada tarjeta tiene 2 instancias con Conexiones máximas 200 000, Mensajes por segundo 200 000 y Conexiones nuevas por segundo 20 000. Eso da 800 000 huecos de conexión para 200 000 espectadores y 800 000 mensajes por segundo para los 100 000 de una tarde normal. Los mensajes enviados van a 2 servidores de aplicaciones Go y a DynamoDB con 16 particiones.

Cuesta 13 759 $ al mes. CloudFront es casi todo, 12 537 $, porque se factura por petición, a 4750 descargas de segmentos por segundo. Después vienen DynamoDB 412 $, S3 365 $, 140 $ por cada tarjeta de gateway, los servidores de aplicaciones 140 $ y el ALB 25 $.

«Una tarde normal» se supera sin errores y con un p95 de alrededor de 1 ms: casi todas las descargas son aciertos de la CDN.

«Una raid inunda un canal» multiplica el tráfico por cuatro a partir del segundo 20, durante 20 segundos: cuatro veces las descargas de segmentos, los mensajes enviados y los mensajes que recibe cada espectador, 400 000 por segundo en total. Se supera sin errores. Los gateways pueden enviar 800 000 mensajes por segundo, así que la raid ocupa la mitad. La CDN absorbe las descargas de más, porque todos los que llegan con la raid ven los mismos segmentos.

«El almacenamiento de origen se vuelve lento» añade 400 ms a cada llamada a S3 a partir del segundo 20, durante 30 segundos. Se supera sin errores: la CDN responde casi todas las descargas, y solo los fallos de caché ven el origen lento.

Aquí no hay reto de coste. El mejor diseño más barato que se encontró costaba el 98 % de la referencia. La factura de la CDN manda, y cada descarga de segmento la necesita.

## Por qué falla el primer intento

Un primer intento habitual es el mismo diseño con una sola tarjeta de gateway para el chat (2 instancias con Conexiones máximas 150 000 y Mensajes por segundo 100 000 cada una) y sin variación al reconectar. Son 300 000 huecos y 200 000 mensajes por segundo. Cuesta 13 619 $ al mes, y «Una tarde normal» sale Aceptada: 100 000 mensajes por segundo caben en 200 000 por segundo de capacidad.

«Una raid inunda un canal» falla con un p95 de 19 s. 400 000 mensajes por segundo se encuentran con 200 000 por segundo de capacidad, así que crece un atasco durante los 20 segundos de la raid, unos 4 000 000 de mensajes, y sigue vaciándose mucho después. El chat llega tarde a todos, no solo al canal de la raid. Las conexiones están bien; lo que no cabe son los mensajes.

La idea es esta: el vídeo en directo es casi todo lectura en el borde, así que lo lleva la CDN, y el origen apenas importa. El chat es un problema de difusión, así que dimensiona los gateways por mensajes por segundo, con margen para el momento en que una raid los multiplica.

## Lo que el simulador deja fuera

No se simulan la codificación del vídeo ni el camino de entrada (la subida del streamer y la transcodificación), ni las escaleras de bitrate. Quedan fuera el tamaño de los segmentos en bytes y el precio del ancho de banda de la CDN: la CDN se factura solo por petición. No hay salas de chat por canal: cada espectador recibe mensajes al ritmo medio de la plataforma, no solo los del canal que ve. Tampoco se simula la moderación.

## Para probar

- Quita la CDN CloudFront de la referencia, conecta el Cliente directamente a S3 y ejecuta «Una tarde normal». ¿Qué le pasa al origen?
- Quita la segunda tarjeta de gateway de la referencia y ejecuta «Una raid inunda un canal». ¿Bastan los 400 000 mensajes por segundo de una tarjeta, sin margen?
- Baja el TTL de la CDN a 1 s en la referencia y ejecuta «El almacenamiento de origen se vuelve lento». ¿Ven ahora más descargas el origen lento?
