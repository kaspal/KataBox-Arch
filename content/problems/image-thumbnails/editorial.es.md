# Solución comentada

## Adónde va la carga

Con 300 subidas/s de 2 MB cada una, entran en S3 unos 600 MB/s de originales. Guardarlos le cuesta poco a la Upload API; el coste real está en redimensionar. Cada trabajo ocupa un hueco de Worker unos 200 ms para decodificar y redimensionar, más unos 50 ms para escribir las miniaturas en S3. 300 trabajos por segundo × 0,25 s ≈ 75 huecos ocupados todo el tiempo en el pico.

Con una cola en medio, la Upload API responde rápido y los trabajos esperan su turno. La cola solo ayuda si los Workers tienen margen sobre esos 75 huecos y si perder una parte de ellos no detiene el trabajo.

## El diseño de referencia

Un Load Balancer (AWS ALB) envía las subidas a la Upload API, con 2 instancias Go. Esta escribe el original en S3 y publica un trabajo en una Queue SQS con sus valores por defecto. Dos tarjetas de Workers Celery consumen esa cola: Thumbnailers A y Thumbnailers B, con 12 instancias cada una y la concurrencia por defecto de 8, así que 96 huecos por tarjeta y 192 en total. Cuesta unos 3786 $ al mes, peticiones a S3 y SQS incluidas, con un p95 de cerca de 0,7 s en un día normal.

«Una avalancha de subidas» manda 3 veces el tráfico durante 10 segundos, a partir del segundo 20, y admite un p95 de 20 s. 900 trabajos por segundo necesitan unos 225 huecos, algo más de los 192 que hay, así que se forma una cola corta que se vacía pocos segundos después de la avalancha. El p95 queda en unos 4 s.

«Se cae un worker» tumba la primera tarjeta de Workers, Thumbnailers A, en el segundo 20 durante 30 segundos, en una ejecución de 120 segundos. Admite un p95 de 20 s y ningún mensaje perdido. Los 96 huecos de Thumbnailers B bastan por sí solos para 300 trabajos por segundo. Los trabajos que A tenía cuando cayó no se pierden: SQS presta cada mensaje a un Worker, y cuando el visibility timeout de 30 segundos se agota sin respuesta, el mensaje vuelve a la cola para que lo tome B. El p95 se queda en cerca de 0,7 s.

## Por qué falla el primer intento

Un primer intento habitual es una sola tarjeta de Workers, Thumbnailers, con 16 instancias: 128 huecos, más de los 75 que pide el pico, por unos 3226 $ al mes. Supera «Un día normal», y también la avalancha, con un p95 de unos 11 s, todavía por debajo del límite.

Donde falla es en la caída. Esa única tarjeta es la primera tarjeta de Workers, así que el fallo detiene todas las miniaturas durante 30 segundos. No se pierde ningún mensaje, porque el préstamo devuelve cada trabajo, pero se acumulan unos 9000 trabajos en la cola. Cuando la tarjeta vuelve, solo le sobran unos 50 huecos por encima de la carga que sigue entrando para vaciarla, y eso lleva bastante más de medio minuto. El p95 llega a unos 28 s, por encima del límite de 20 s.

La lección: los préstamos hacen que una caída cueste tiempo en vez de datos. Divide los Workers para que una sola caída no pueda detener el trabajo, y dale a cada parte margen suficiente para llevar la carga sola.

## Para probar

- Borra Thumbnailers B y ejecuta «Se cae un worker». Después vuelve a ponerla con 6 instancias y mira si media tarjeta basta.
- Baja el visibility timeout de la cola a 10 s y ejecuta la prueba de la caída. Los trabajos que A no terminó vuelven antes a B; en el primer intento cambia poco, porque no queda ningún Worker para tomarlos.
- Reduce las dos tarjetas a 8 instancias y observa el p95 durante «Una avalancha de subidas».
