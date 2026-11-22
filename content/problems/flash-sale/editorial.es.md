# Solución comentada

## Adónde va la carga

Antes de la venta, 2 000 peticiones/s se reparten en unas 1 800 visitas a páginas y 200 compras. El problema empieza cuando se abre la venta: el tráfico se multiplica por 25 durante 10 segundos, hasta unas 50 000 peticiones por segundo. Unas 45 000 son personas recargando la página del evento, y solo hay 200 páginas, así que una CDN puede responder casi todas. Las otras 5 000 por segundo son pedidos, más de los que acepta el proveedor de pagos. No se pueden cobrar todos al instante, y no hace falta. Hay que guardarlos y cobrarlos en orden.

## El diseño de referencia

El Client habla con una CDN con un TTL de 60 segundos. Los fallos de página van a Object Storage (S3). La CDN reenvía el pago a un Application Load Balancer delante de 4 servidores Go de la Checkout API. Cada servidor pone el pedido en una Queue SQS con un max depth de 1 000 000 y responde al comprador. Los Payment Workers, 8 instancias con 64 huecos cada una, toman pedidos de la Queue, cobran a través del proveedor y escriben la entrada en PostgreSQL.

Cuesta unos 5 429 $ al mes. El p95 se mantiene cerca de 25 ms durante la apertura, porque la respuesta al comprador solo espera a la Queue. Cuando el proveedor de pagos se ralentiza y añade 3 segundos a cada cobro durante 30 segundos, los Workers vacían la cola más despacio y la Queue se alarga. Ningún comprador espera y no se pierde ningún pedido.

## Por qué falla el primer intento

Un primer intento habitual es un Load Balancer delante de 4 servidores Go de la tienda que sirven ellos mismos las páginas de los eventos y además atienden el pago, alimentando una Queue RabbitMQ con un max depth de 5 000 y los mismos Payment Workers. Antes de la venta pasa con un p95 cercano a 18 ms. Cuando se abre la venta, todas las visitas a páginas caen sobre esos 4 servidores. Se quedan sin huecos, las peticiones se acumulan unas detrás de otras y falla alrededor del 17 % de las peticiones. Las visitas ahogan a las compras con las que comparten servidores.

## Para probar

- Mantén la CDN, pero reduce el max depth de la Queue a 5 000. Ejecuta la apertura y cuenta los pedidos perdidos.
- Quita la CDN y añade servidores de la tienda hasta que la apertura pase. Compara el coste.
- Reduce los Payment Workers a 2 instancias y ejecuta el proveedor lento. ¿Cuánto tarda en vaciarse la acumulación?
