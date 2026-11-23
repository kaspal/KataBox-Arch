## 1

La mayoría de las peticiones son mosaicos del mapa, y los mismos pocos mosaicos se piden una y otra vez. Después pregúntate: el feed de tráfico solo mejora un poco la hora de llegada. ¿Qué debería hacer una ruta cuando el feed deja de responder?

## 2

Pon una CDN delante de los mosaicos para que la mayoría nunca llegue al almacenamiento de objetos. Para las rutas, una llamada opcional solo es opcional si tus servidores dejan de esperarla: un tiempo límite de llamada corto acota cuánto ocupa un hueco del servidor una llamada lenta, y un circuit breaker deja de llamar a un feed que no para de fallar, así que las rutas vuelven enseguida con horas de llegada de velocidades típicas.

## 3

Conecta el Cliente a una CDN CloudFront con un TTL de 86 400 s (un día), delante de Amazon S3. Envía las rutas por un AWS ALB a Servidores de aplicaciones Go con 5 instancias, el Tiempo límite de llamada en 150 ms y el Cortocircuito con errores al 50 %, y conéctalos a una Base de datos de grafos Neo4j con 3 instancias.
