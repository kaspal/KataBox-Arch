# Solución comentada

## Adónde va la carga

Con 4 000 peticiones/s, unas 3 960 son reproducciones de segmentos y unas 40 son subidas. Las reproducciones son baratas una a una, pero son muchas y muy repetitivas: con Zipf 1,1 sobre 500 000 segmentos, los mismos segmentos populares se piden una y otra vez. Un segmento no cambia nunca una vez escrito, así que es ideal para una CDN con un TTL largo. Las subidas son lo contrario: pocas, pero cada una necesita unos 400 ms de transcodificación. Ese trabajo no debe estar en el camino de nada que un espectador esté esperando.

## El diseño de referencia

El Client habla con una CDN (CloudFront) con un TTL de un día. Los fallos de segmentos van a Object Storage (S3), que guarda todas las versiones. La CDN reenvía las subidas a un Load Balancer delante de 2 servidores Go de la Upload API. Cada subida se escribe en S3 y se publica en una Queue SQS, y 8 Transcoder Workers vacían la Queue y escriben las versiones de vuelta.

Cuesta unos 12 515 $ al mes, la mayor parte en peticiones a la CDN. El p95 ronda los 7 ms, porque casi todas las reproducciones terminan en el borde. Aguanta el estreno de la temporada, que envía 10 veces el tráfico durante 20 segundos con segmentos aún más calientes (Zipf 1,4): unos segmentos más calientes solo suben la tasa de aciertos de la CDN. También aguanta la avalancha de subidas, cuando las subidas llegan a 6 veces su ritmo normal: la Queue se alarga un poco y los espectadores no lo notan.

## Por qué falla el primer intento

Un primer intento habitual es un Load Balancer delante de 3 servidores de vídeo Go que sirven los segmentos ellos mismos y además atienden las subidas. En una tarde normal pasa con un p95 cercano a 17 ms. En el estreno, 40 000 peticiones por segundo caen sobre 3 servidores sin un borde que las absorba. Se quedan sin huecos, el trabajo se acumula y falla alrededor del 24 % de las peticiones.

## Para probar

- Baja el TTL de la CDN a unos pocos segundos y observa la tasa de aciertos y la carga del origen en la prueba visible.
- Sustituye S3 detrás de la CDN por los 3 servidores Go. ¿Sigue pasando el estreno?
- Reduce los Transcoder Workers a 2 y ejecuta la avalancha de subidas. ¿Quién espera, y falla alguien?
