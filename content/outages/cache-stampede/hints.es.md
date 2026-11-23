## 1

La base de datos iba bien antes del vaciado, y la caché sí se rellena. Entonces, ¿por qué hay tantos errores en los primeros segundos, y por qué vuelven unos 30 segundos después?

## 2

Mientras se responde el primer fallo de un producto, todas las demás peticiones de ese producto también fallan y lanzan la misma consulta de 200 ms. Un producto popular lanza cientos de copias del mismo trabajo. Solo una tiene que llegar a la base de datos.

## 3

En la caché Redis, activa la protección de estampida (agrupar fallos). No hace falta cambiar nada más.
