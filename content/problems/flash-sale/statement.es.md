# Venta flash de entradas

## El producto

Vendes entradas para grandes eventos. La mayoría de los días son tranquilos: la gente mira las páginas de los eventos y de vez en cuando compra un asiento. Luego un concierto popular sale a la venta a una hora fija, y una multitud que llevaba un rato esperando con la página abierta llega toda junta. Comprar una entrada supone cobrar al comprador a través de un proveedor de pagos externo, que es lento y tiene su propio límite de ritmo. El comprador no necesita que el cobro termine para ver una respuesta, solo saber que el pedido se ha recibido.

## Lo que debes cumplir

| Objetivo | Valor |
| --- | --- |
| Tráfico pico antes de la venta | 2 000 peticiones/s |
| Navegación | 90 % de las peticiones |
| Latencia p95 | 100 ms o menos |
| Disponibilidad | 99,9 % o más |
| Presupuesto | 9000 $ al mes |

## El tráfico

- **Ver la página del evento** (90 %). La petición puede pasar por un componente de borde, como una CDN, y luego la sirve un origen: un App Server u Object Storage. Hay 200 páginas de eventos y todas se pueden cachear.
- **Comprar una entrada** (10 %). La petición puede pasar por el borde y luego llega a la Checkout API, en un App Server, que pone el pedido en una Queue. El comprador recibe una respuesta en cuanto el pedido está en la cola. Después, los Payment Workers cobran a través del proveedor de pagos, que tarda unos 300 ms de mediana y acepta hasta 2 000 cobros por segundo, y escriben la entrada en la base de datos.

## Las pruebas

Hay una prueba visible. Puedes ejecutarla tantas veces como quieras:

- **Antes de la venta**: el tráfico descrito arriba, en su pico.

Hay dos pruebas ocultas más, de las que solo ves el nombre: **Se abre la venta** y **El proveedor de pagos se ralentiza**. Tu diseño tiene que superar las tres.
