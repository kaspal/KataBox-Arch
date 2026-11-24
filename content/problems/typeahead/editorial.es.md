# Solución comentada

## Adónde va la carga

Cada una de las 6 000 peticiones/s es una lectura, y están muy desequilibradas. Con Zipf 1,1 sobre 200 000 prefijos, unos pocos prefijos cortos, como las letras sueltas, se llevan una gran porción de todas las pulsaciones. El índice de Search coloca cada clave en un shard, y la misma clave siempre cae en el mismo shard. Así, el prefijo más caliente envía toda su carga a un solo shard mientras los demás quedan casi ociosos. Más shards separan más las claves distintas, pero nunca dividen una clave caliente. La respuesta para un prefijo popular apenas cambia en una hora, y justo para eso sirve una caché.

## El diseño de referencia

Un Load Balancer reparte el tráfico entre 3 App Servers Go. Cada App Server consulta primero una Cache Redis llamada «prefixes» (2 GB, TTL de una hora). Un acierto termina la petición. Un fallo pregunta a Elasticsearch, con 4 shards, y llena la caché.

Cuesta unos 759 $ al mes y su p95 ronda los 18 ms en todas las pruebas. Casi todos los prefijos se responden desde la caché, así que el índice solo ve un hilo fino de prefijos raros, repartido de forma pareja entre sus shards.

## Por qué falla el primer intento

Un primer intento habitual son los mismos App Servers llamando a Elasticsearch con 12 shards en cada pulsación. Parece generoso, pero incluso en un día normal los prefijos más populares se amontonan en un shard. Ese shard se satura, los hilos de los App Servers esperan detrás y falla alrededor del 42 % de las peticiones.

Las pruebas ocultas lo empeoran. «Todos escriben lo mismo» triplica el tráfico durante 20 s y hace los prefijos calientes mucho más calientes (Zipf 1,5), así que el shard caliente recibe todavía más carga. «El índice se reconstruye» suma 200 ms a cada petición a Search durante 40 s, lo que por sí solo supera de lejos el límite de 60 ms cuando cada pulsación llega al índice. Con la caché delante, ambas pruebas solo afectan a los pocos fallos, y el p95 no se mueve.

## Para probar

- Quita la Cache de la referencia y cambia el número de shards de 4 a 12. Observa el shard más ocupado, no la media.
- Baja el TTL a un minuto y compara la tasa de aciertos y el p95.
- Reduce la Cache y busca el tamaño a partir del cual los fallos empiezan a llegar al índice en gran número.
