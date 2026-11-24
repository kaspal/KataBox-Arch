# Solución comentada

## Adónde va la carga

Con 60 000 peticiones/s y un 99,9 % de lecturas, unas 59 940 peticiones/s leen un flag y solo unas 60 lo cambian. Con 20 000 flags, cada uno cambia de media una vez cada 333 segundos, mientras que los populares se leen miles de veces por segundo. Los 20 000 flags de 1 KB caben en unos 20 MB. Es la forma ideal para una caché: pequeño, muy leído y casi sin cambios.

Sin caché, cada lectura es una consulta a la base de datos. Un miembro de PostgreSQL trabaja con 100 consultas a la vez a unos 5 ms cada una, unas 20 000 por segundo, así que ni la principal y una réplica juntas se acercan a 60 000. Los App Servers también lo notan: cada uno conserva su hueco mientras espera a la caché y a la base de datos, así que una base de datos lenta llena sus huecos.

## El diseño de referencia

Un Load Balancer ALB reparte el tráfico entre 10 App Servers Go, la Flags API. Cada uno lee primero una Cache Redis con el TTL en 3 segundos; un fallo de caché lee PostgreSQL y llena la caché. PostgreSQL funciona con una principal y una réplica unidas por una arista Replicate; los cambios van a la principal. Cuesta unos 1269 $ al mes, con un p95 cercano a 20 ms y cerca de un 1 % de lecturas obsoletas en un día normal, casi todas por el retraso de la réplica.

El TTL es la lección. Con 3 segundos, un flag en caché nunca va más de 3 segundos por detrás de su último cambio, y a la base de datos solo le llega un pequeño flujo de fallos de caché. «Una tormenta de cambios» multiplica los cambios por 5, hasta unos 300 por segundo, así que cada flag cambia más o menos una vez por minuto. Caen más lecturas en la ventana que sigue a un cambio y las lecturas obsoletas suben a cerca del 2,4 %, todavía por debajo del límite del 5 %. Con 5 segundos llegarían a cerca del 5,4 %, justo por encima. «La base de datos de configuración cae 20 s» mata la PostgreSQL principal en el segundo 20 durante 20 segundos, y admite un 1 % de errores y un 99 % de disponibilidad. La caché sigue respondiendo por los flags que guarda, así que solo dan error algunas lecturas que no estaban en caché y los cambios: alrededor del 0,17 % de las peticiones.

## Por qué falla el primer intento

Un primer intento habitual es el mismo diseño sin la caché: el ALB, 10 App Servers Go y la PostgreSQL principal con su réplica. Cada lectura se convierte en una consulta. Los dos miembros no pueden con 60 000 consultas por segundo, los huecos de los App Servers se acumulan esperándolos y falla alrededor del 49 % de las peticiones incluso en «Un día normal», con un p95 cercano a 205 ms. La tormenta de cambios no va mejor, y con la principal caída falla cerca del 59 %. Sus lecturas casi nunca son obsoletas, porque no hay copia en caché que envejezca, solo un poco de retraso de la réplica, pero la mitad no llega nunca.

## Para probar

- Devuelve el TTL a sus 300 segundos por defecto y ejecuta «Una tormenta de cambios». Ahora un cambio tarda minutos en verse y se supera el límite de lecturas obsoletas.
- Pon el TTL en 1 segundo y ejecuta «La base de datos de configuración cae 20 s». Llegan más fallos de caché a la base de datos, y más de ellos fallan mientras está caída.
- Baja los App Servers a 5 y comprueba si el p95 aguanta.
