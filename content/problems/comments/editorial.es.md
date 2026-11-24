# Solución comentada

## Adónde va la carga

Con 6000 peticiones por segundo, unas 5760 son lecturas y unas 240 son comentarios nuevos. En una tarde normal la popularidad es una Zipf de 0,9 sobre 100 000 hilos: los hilos populares reciben más, pero las lecturas siguen repartidas entre muchas claves. Casi cualquier almacén aguanta eso.

La prueba viral cambia la forma, no solo el tamaño. El tráfico se triplica hasta 18 000 peticiones por segundo, y la popularidad se afila hasta una Zipf de 1,4. Con ese sesgo, el hilo más popular se lleva por sí solo cerca de un tercio de todo (1/ζ(1,4) ≈ 32 %): unas 5800 peticiones por segundo para una sola clave. Un almacén particionado coloca una clave en una partición, así que toda esa carga cae en una máquina, tenga las particiones que tenga.

## El diseño de referencia

Un AWS ALB reparte el tráfico entre 4 Go App Servers. Leen de una Redis Cache de 4 GB con un TTL de 300 s, y en un fallo de caché leen de Cassandra con 1 partición. Un comentario nuevo se escribe en Cassandra, y después el App Server refresca la entrada de ese hilo en la caché. Cuesta unos 604 $ al mes: 280 $ los App Servers, 250 $ Cassandra, 49 $ Redis y 25 $ el ALB. Una tarde normal tiene un p95 de unos 18,6 ms, ningún error y un 0,09 % de lecturas obsoletas. Cassandra solo está ocupada en torno a un 4 %: ve los fallos de caché y los comentarios, no las lecturas.

«Un hilo se hace viral» triplica el tráfico a partir del segundo 15, durante 30 segundos, con una popularidad Zipf de 1,4. Redis responde el hilo caliente desde memoria, así que el p95 se queda en unos 18,6 ms, sin errores. Las lecturas obsoletas suben al 6,65 %: el hilo caliente recibe comentarios nuevos cada segundo, y una lectura puede caer entre la escritura en Cassandra y el refresco de la caché. Aquí eso no es un problema; un comentario que aparece un momento tarde no hace daño.

«Se vacía la caché en una tarde con mucho tráfico» duplica el tráfico con una popularidad Zipf de 1,2 y vacía la caché en el segundo 20. Los hilos calientes fallan una vez y vuelven a la caché en milisegundos, y Cassandra, ocupada normalmente en un 4 %, tiene sitio para los fallos fríos que vienen después. El p95 queda en unos 19,0 ms, sin errores.

Este problema no tiene reto de coste. El mejor diseño más barato que se encontró seguía costando cerca del 82 % de la referencia, por encima del objetivo del 80 %: la Redis Cache que sostiene la lección ya es una parte pequeña de la factura.

## Por qué falla el primer intento

Un primer intento habitual es el mismo borde y los mismos App Servers sin caché, y Cassandra con 2 particiones, dimensionada para una tarde normal: cada partición está ocupada en torno a un 34 %. Cuesta unos 805 $, más que la referencia, y supera «Una tarde normal» con un p95 de unos 22,7 ms.

La prueba viral lo rompe: se descartan cerca de un 3,6 % de las peticiones, y el p95 sube a unos 62 ms, por encima del límite de 60 ms. El hilo viral vive en una partición, porque la clave decide la partición. Esa partición recibe las ~5800 peticiones por segundo del hilo más la mitad de todo lo demás, unas 12 000 por segundo, mientras que la otra recibe unas 6000. Añadir particiones no ayuda: reparten más finamente los demás hilos, pero una clave sigue teniendo una sola casa. La prueba del vaciado se supera, porque no hay caché que vaciar.

La idea es esta: una caché absorbe una clave caliente, porque un nodo de Redis la responde desde memoria, y particionar no puede repartir una sola clave. Además, el diseño con caché es más barato, porque el almacén solo ve los fallos de caché.

## Lo que el simulador deja fuera

No se simulan las réplicas de Cassandra ni los niveles de consistencia: replication_factor y consistency no afectan a la ejecución. Tampoco se simulan las reglas reales para particiones calientes, como la capacidad adaptativa de DynamoDB. Se dejan fuera el orden de los comentarios y la paginación: cada lectura es una clave. La memoria de la caché por hilo es solo el tamaño de su valor, y los App Servers no tienen cachés locales propias.

## Para probar

- Quita la Redis Cache, sube Cassandra a 4 particiones y ejecuta la prueba viral. Mira si la partición caliente se enfría algo.
- Baja el TTL de la caché a 10 s y ejecuta la prueba visible. Compara lo ocupada que está Cassandra con la referencia.
- Reduce a la mitad la memoria de la caché varias veces y ejecuta la prueba del vaciado tras cada paso. Mira la tasa de aciertos y lo ocupada que está Cassandra.
