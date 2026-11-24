# Solución comentada

## Adónde va la carga

Con 12 000 peticiones/s y un 95 % de lecturas, unas 11 400 peticiones/s son redirecciones y unas 600 son enlaces nuevos. Las lecturas son todo el problema, y están muy desequilibradas: con Zipf 1,1 sobre 10 millones de enlaces, una porción pequeña de enlaces calientes recibe la mayoría de los clics. Justo para eso sirve una caché. Si la mayoría de las redirecciones terminan en la caché, la base de datos solo ve fallos de caché y escrituras, y los hilos de los App Servers pasan mucho menos tiempo esperándola.

## El diseño de referencia

Un Load Balancer (NGINX) reparte el tráfico entre 8 App Servers Go. Cada App Server consulta primero una Cache Redis de 8 GB. Un acierto termina la petición; un fallo lee de PostgreSQL y llena la caché. En 8 GB caben unos 8 millones de entradas de 1 KB, mucho más que el conjunto caliente. PostgreSQL funciona con una principal y una réplica unidas por una arista Replicate, con un tiempo de failover de 10 segundos. Las lecturas se reparten entre ambas; las escrituras van a la principal.

Cuesta unos 1203 $ al mes y su p95 ronda los 22 ms en un día normal. Aguanta el pico viral. Cuando la principal se cae, alrededor del 1,1 % de las peticiones falla mientras dura el failover; luego la réplica asciende y el tráfico se recupera.

## Por qué falla el primer intento

Un primer intento habitual es un Load Balancer, 4 App Servers Go y una sola base de datos PostgreSQL, sin caché ni réplica. Supera las dos pruebas visibles con un p95 cercano a 29 ms, así que parece terminado.

Las pruebas ocultas encuentran dos huecos. «Un enlace se hace viral» envía 4 veces el tráfico durante 15 segundos, con un límite de p95 de 120 ms. Cada clic cae en la única base de datos, que se satura; los hilos de los App Servers se acumulan detrás y falla alrededor del 39 % de las peticiones. «Se cae la base de datos principal» mata la principal SQL a los 30 segundos y admite hasta un 2 % de errores. Sin réplica no hay nada que ascender: desde ese momento falla toda petición que necesite la base de datos, cerca del 50 % de toda la ejecución.

## Para probar

- Borra la réplica y vuelve a ejecutar «Se cae la base de datos principal». Después baja el tiempo de failover y compara la tasa de errores.
- Reduce la Cache a 1 GB y observa la tasa de aciertos y el p95 en «Un enlace que todos pulsan».
- Quita App Servers de uno en uno y busca el número mínimo que todavía resiste el pico viral.