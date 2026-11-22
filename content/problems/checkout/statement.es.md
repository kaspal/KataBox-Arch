# Checkout e inventario

## El producto

Tienes una tienda online. La mayoría de las visitas son para mirar: abren fichas de productos, comparan y abren unas cuantas más. Algunas compran. Una compra guarda el pedido, reserva el stock para que el artículo no se venda dos veces y cobra la tarjeta a través de un proveedor de pagos externo. Tú no gestionas ese proveedor, y en un mal día se vuelve lento. Haga lo que haga, la tienda tiene que seguir respondiendo, y un pedido que hizo un cliente nunca debe desaparecer sin más.

## Lo que debes cumplir

| Objetivo | Valor |
| --- | --- |
| Tráfico pico | 4000 peticiones/s |
| Lecturas | 80 % de las peticiones |
| Latencia p95 | 800 ms o menos |
| Disponibilidad | 99,9 % o más |
| Presupuesto | 4000 $ al mes |
| Mensajes perdidos | Ninguno |

La latencia se mide hasta que el cliente recibe una respuesta.

## El tráfico

Los clientes llegan a la Shop API, un App Server, normalmente a través de un Load Balancer.

- **Ver productos** (80 %). El App Server lee el producto, primero de una Cache si tu diseño la tiene y, si no, de la base de datos. Hay 50 000 productos y su popularidad sigue una curva Zipf 1,1: un grupo pequeño de productos se lleva la mayoría de las visitas.
- **Hacer un pedido** (20 %). El App Server escribe el pedido y reserva el stock en la base de datos principal. Después se cobra la tarjeta a través del proveedor de pagos, que responde en unos 300 ms de mediana, con una cola lenta que llega a 1,5 s. El cobro se puede pasar a una cola, de donde lo toma un Worker que llama al proveedor. Si el App Server no tiene una cola en la que publicar, llama él mismo al proveedor, conserva su hueco durante toda la llamada y el cliente espera.

Si guardas los productos en caché, una ficha puede mostrar un stock con hasta una vida de caché de antigüedad mientras los pedidos lo cambian. Aquí no importa: las lecturas desactualizadas no se evalúan, porque el propio pedido reserva contra la base de datos.

## Las pruebas

Hay una prueba visible. Puedes ejecutarla tantas veces como quieras:

- **Un día normal**: el tráfico descrito arriba, en su pico.

Hay dos pruebas ocultas más, de las que solo ves el nombre: **El proveedor de pagos se ralentiza** y **Tormenta de doble clic (pedidos ×2)**. Tu diseño tiene que superar las tres.
