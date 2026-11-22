Una caché guarda en memoria una copia de los datos que se leen con frecuencia, cerca de tus servidores de aplicaciones. Una lectura que encuentra ahí su clave (un **acierto**) responde en mucho menos de un milisegundo. Una lectura que no la encuentra (un **fallo**) pasa a la base de datos, y la respuesta se guarda en la caché a la vuelta, así que la siguiente lectura acierta.

Cambia un poco de frescura por mucha velocidad, y quita carga de lectura a la base de datos, que suele ser la parte más difícil de escalar de un sistema.

Lo bien que funciona depende del tráfico. Cuando unas pocas claves se llevan la mayoría de las lecturas, una caché pequeña atrapa casi todas. Cuando todas las claves son igual de probables, la caché tiene que ser casi tan grande como los datos para ayudar.
