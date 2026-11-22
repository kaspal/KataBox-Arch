## 1

¿Cuántas veces se descarga el mismo segmento popular en un minuto, y de verdad tu origen tiene que servirlo cada vez?

## 2

Pon los segmentos detrás de una CDN para que los populares se respondan en el borde, y saca la transcodificación del camino de la petición: basta con guardar y encolar la subida antes de responder al creador.

## 3

Coloca una CDN con un TTL largo (un día está bien, los segmentos nunca cambian) delante de un Object Storage que guarde los segmentos. Haz que la CDN reenvíe las subidas a un Load Balancer y a un par de servidores de la Upload API, que escriben en Object Storage y publican en una Queue que vacían varios Transcoder Workers.
