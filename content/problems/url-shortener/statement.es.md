# Acortador de URLs

## El producto

Tienes un servicio que convierte direcciones web largas en cortas. Alguien pega un enlace largo y recibe un código corto para compartir. Casi todo el tráfico llega por el otro lado: personas que siguen esos enlaces cortos y esperan llegar a la página original al instante. Cada redirección es pequeña, pero hay muchísimas, y unos pocos enlaces populares reciben muchos más clics que el resto.

## Lo que debes cumplir

| Objetivo | Valor |
| --- | --- |
| Tráfico pico | 12 000 peticiones/s |
| Lecturas | 95 % de las peticiones |
| Latencia p95 | 80 ms o menos |
| Disponibilidad | 99,9 % o más |
| Presupuesto | 2500 $ al mes |

## El tráfico

- **Seguir un enlace corto** (95 %). La petición puede pasar por un componente de borde, como un Load Balancer, y luego llega a un App Server. El App Server puede consultar primero una Cache y, si no encuentra el enlace, lee el almacén de datos. Hay 10 millones de enlaces y su popularidad sigue una curva Zipf 1,1: un grupo pequeño de enlaces se lleva la mayoría de los clics.
- **Crear un enlace corto** (5 %). El App Server escribe el enlace nuevo en la base de datos principal.

## Las pruebas

Hay dos pruebas visibles. Puedes ejecutarlas tantas veces como quieras:

- **Un día normal**: el tráfico descrito arriba, en su pico.
- **Un enlace que todos pulsan**: el mismo volumen, con los clics aún más concentrados en los enlaces más populares.

Hay dos pruebas ocultas más, de las que solo ves el nombre: **Un enlace se hace viral** y **Se cae la base de datos principal**. Tu diseño tiene que superar las cuatro.
