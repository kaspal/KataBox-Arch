# Solución comentada

## Adónde va la carga

En el pico, las 50 claves de clientes envían unas 2 970 peticiones/s entre todas, cerca de 59 cada una. La clave abusiva añade unas 30 más. Es poco trabajo para dos App Servers y una base de datos PostgreSQL. El peligro no es el total, sino que una sola clave puede multiplicar su ritmo sin avisar. En «Una clave inunda la API» la clave abusiva envía 300 veces su ritmo habitual, unas 9 000 peticiones/s extra. Todo lo que hay detrás del borde es compartido, así que si esas llamadas entran, se quedan con los hilos que necesitan los clientes reales.

## El diseño de referencia

Un Rate Limiter (Envoy) en el borde, con un límite de 200 peticiones/s por clave y una ráfaga de 400. Detrás hay 2 App Servers Go y una base de datos PostgreSQL. El diseño completo cuesta unos 520 $ al mes.

Como el Rate Limiter mantiene una cubeta de tokens por clave, la inundación solo vacía la cubeta de la clave abusiva. Sus llamadas extra se descartan en el borde, lo que el juez cuenta como un éxito, y los App Servers nunca las ven. Las otras 50 cubetas siguen llenas.

En «Día de lanzamiento de todos los clientes (×2 durante 15 s)», cada cliente llega a unas 119 peticiones/s. Eso queda holgadamente por debajo del límite de 200, así que no se rechaza a ningún cliente real.

## Por qué falla el primer intento

El primer intento habitual pone un Load Balancer delante de 2 App Servers Go y PostgreSQL. Pasa un día normal. Durante la inundación, el Load Balancer reenvía todas las llamadas, los App Servers se saturan y falla alrededor del 22 % de las peticiones de todos. Los clientes honestos pagan por el comportamiento de una sola clave.

El error contrario es un límite demasiado ajustado. Un Rate Limiter a 100 peticiones/s por clave parece seguro en un día normal, porque 59 está por debajo de 100. El día de lanzamiento cada cliente necesita unas 119, así que el limitador empieza a descartar llamadas legítimas, y cada una cuenta como un error. Deja margen por encima del momento más intenso de un cliente real, no de su media.

## Para probar

- Baja el límite por clave a 100 y ejecuta el día de lanzamiento. Después prueba con 150.
- Pon la ráfaga en 0 y mira si empiezan a fallar los picos cortos de los clientes reales.
- Cambia el Rate Limiter por un Load Balancer y añade App Servers hasta que pase la inundación. Compara el coste.