# Carrito de la compra

## El producto

Llevas el servicio de carrito de una gran tienda online. Los compradores miran su carrito, añaden y quitan artículos, y pagan. Hay 2 000 000 de carritos. La mayoría de los días son tranquilos, pero las rebajas y las ofertas relámpago traen multitudes de golpe, y la capa de aplicación escala para atenderlas. Un cambio en un carrito no se puede perder nunca: quien añade un artículo espera verlo ahí al pagar.

## Lo que debes cumplir

| Objetivo | Valor |
| --- | --- |
| Tráfico pico | 6000 rps |
| Latencia p95 | 150 ms o menos |
| Disponibilidad | 99,9 % o más |
| Tasa de errores | 1 % o menos |
| Presupuesto | 1800 $ al mes |
| Escrituras perdidas | Ninguna |

## El tráfico

Las peticiones entran por un borde opcional hasta tus servidores de aplicaciones. Cada petición toca uno de los 2 000 000 de carritos.

- **Ver el carrito** (60 %). La aplicación lee el carrito, primero de una caché si tienes una, y de la base de datos si falla.
- **Añadir o quitar un artículo** (30 %). La aplicación escribe el cambio en el primario de la base de datos y después actualiza el carrito en la caché si tienes una.
- **Pagar** (10 %). La aplicación escribe el pedido en el primario de la base de datos.

Este problema cuenta las conexiones a la base de datos. PostgreSQL acepta como mucho 1700 conexiones (su max_connections, la cifra de una RDS db.m5.xlarge). Cada hueco de la aplicación que habla con la base directamente ocupa una conexión, así que una tarjeta de Servidor de aplicaciones ocupa instancias × concurrencia conexiones. Las conexiones por encima del límite se rechazan, y las peticiones que las necesitaban fallan.

## Las pruebas

Hay una prueba visible. Puedes ejecutarla tantas veces como quieras:

- **Una tarde normal**: el tráfico descrito arriba, en su pico.

Hay dos pruebas ocultas más, de las que solo ves el nombre: **Empiezan las rebajas** y **Una oferta relámpago a mediodía**. Tu diseño tiene que superar las tres.
