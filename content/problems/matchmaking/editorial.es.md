# Solución comentada

## Adónde va la carga

Con 1000 jugadores por segundo, el trabajo es de los emparejadores. Cada jugador ocupa un hueco de emparejador durante unos 200 ms de emparejamiento, así que 1000 × 0,2 s mantienen ocupados unos 200 huecos, antes de la escritura en DynamoDB. 4 instancias Celery con concurrencia 64 dan 256 huecos. Con la escritura incluida, un grupo de 4 emparejadores está ocupado en torno al 99 % con 1000 jugadores por segundo: toma unos 1000 por segundo y no le sobra nada.

Las pruebas ocultas piden ese margen. El sábado por la noche llegan 2000 jugadores por segundo durante 30 segundos, 1000 por segundo más de los que toma un grupo de emparejadores: unos 30 000 jugadores acumulados en la cola. Cuando se apaga una región, lo que corría allí desaparece, y hay que emparejar a sus jugadores en otro sitio.

## El diseño de referencia

Un DNS Route 53, con un TTL de 30 s, manda a los jugadores a dos regiones, us-east y eu-west. Cada región es una pila completa: un AWS ALB, unos Go App Servers con 2 instancias, una cola RabbitMQ, emparejadores Celery con 4 instancias, concurrencia 64 y un tiempo de servicio de 200 ms, y una tabla DynamoDB. Cada región empareja a sus propios jugadores, así que cada una ve unos 500 por segundo. Cuesta unos 2698 $ al mes. DynamoDB es la mayor parte, unos 829 $ por región, facturados por escritura; los emparejadores cuestan 280 $ por región, los App Servers 140 $, RabbitMQ 70 $, los ALB 25 $ y Route 53 11 $.

«Una tarde entre semana» se supera con un p95 de unos 550 ms y sin errores, con los emparejadores ocupados en torno al 50 %.

«Sábado por la noche» duplica el tráfico a partir del segundo 15, durante 30 segundos. Cada región ve ahora unos 1000 jugadores por segundo, que sus emparejadores todavía pueden tomar: el p95 queda en unos 573 ms y no falla nada.

«Se apaga una región» aísla us-east en el segundo 30, durante el resto de una ejecución de 120 segundos. Admite un 8 % de errores, un 92 % de disponibilidad y un p95 de 10 s. Durante unos 2 segundos, hasta que los chequeos de salud del DNS se dan cuenta, los jugadores de us-east se siguen mandando allí y fallan: un 0,80 % de errores en total. Después eu-west toma a todos los jugadores. Sus emparejadores trabajan en torno al 100 %, pero dan abasto, y el p95 queda en unos 608 ms.

Este problema no tiene reto de coste. El diseño más barato que encontramos que supera todas las pruebas costaba en torno al 95 % de la referencia: las escrituras de DynamoDB son la mayor parte de la factura, y cada partida hay que escribirla una vez, sea como sea el resto del diseño.

## Por qué falla el primer intento

Un primer intento habitual es una sola región, us-east, con la misma pila: un AWS ALB, 2 App Servers, RabbitMQ, 4 emparejadores Celery y DynamoDB. Cuesta unos 2173 $ al mes y supera «Una tarde entre semana» con un p95 de unos 684 ms. Pero sus emparejadores están ocupados al 99 %: no hay margen.

El sábado por la noche la cola crece unos 1000 jugadores por segundo más de lo que los emparejadores pueden tomar. Las esperas suben de 5 s a 20 s a lo largo del pico, y el p95 queda en unos 29,5 s. La cola guardó a todos los jugadores, pero no puede hacer que esperen menos: una cola absorbe un pico solo si los workers tienen margen para vaciarla. Cuando se apaga us-east no queda adónde ir, y falla en torno al 75 % de las peticiones.

La idea es esta: da margen a los emparejadores y pon una cola con sus workers en cada región. El emparejamiento ya es regional, porque los jugadores quieren una partida cerca, así que dos regiones reducen a la mitad la carga que ve cada una y convierten la pérdida de una región en una conmutación por DNS.

## Lo que el simulador deja fuera

No se simula la calidad de las partidas: los emparejadores reales buscan por rango de habilidad y pueden esperar más para encontrar una partida mejor. Cada jugador cuesta 200 ms fijos de emparejamiento. Tampoco se simulan los grupos de jugadores, la latencia entre un jugador y una región lejana ni los servidores de juego fijos.

## Para probar

- Baja los emparejadores de eu-west a 2 instancias y ejecuta «Se apaga una región». ¿Puede la región que queda seguir dando abasto?
- En el diseño de una sola región, sube los emparejadores a 8 instancias y ejecuta «Sábado por la noche». Compara su p95 y su coste con la referencia de dos regiones.
- Sube el TTL de Route 53 y vuelve a ejecutar «Se apaga una región». Mira si cambian los errores antes de la conmutación.
