# Bolsa con casamiento de órdenes

## El producto

Llevas el núcleo de una bolsa de valores. Las pasarelas de los brókeres te envían órdenes de compra y de venta de acciones, y tu bolsa casa cada orden con el libro de órdenes de su valor: la lista de órdenes abiertas que esperan un precio. A los operadores les importa la velocidad, y aún más las órdenes lentas: una orden que se queda parada unos cientos de milisegundos mientras el precio se mueve le cuesta dinero a alguien. Cada orden tiene que llegar a su libro; no se puede perder ninguna.

## Lo que debes cumplir

| Objetivo | Valor |
| --- | --- |
| Tráfico pico | 10 000 peticiones/s |
| Latencia p95 hasta completar | 30 ms o menos |
| Latencia p99 hasta completar | 50 ms o menos |
| Disponibilidad | 99,9 % o más |
| Presupuesto | 2800 $ al mes |
| Mensajes perdidos | Ninguno |

## El tráfico

Las pasarelas de los brókeres llegan a una Order gateway, un App Server, normalmente a través de un Load Balancer.

- **Enviar una orden** (100 %). La Order gateway publica la orden en un stream y responde enseguida. Un Worker, el matcher, toma la orden del stream y la casa en el libro de órdenes, que vive en memoria en una Cache. La latencia se mide hasta que la orden llega a su libro, no hasta que responde la pasarela.

Hay 5000 valores, y su negociación sigue una curva Zipf 1.1: unos pocos valores populares se llevan buena parte de las órdenes. No interviene ningún proveedor externo.

Es el primer problema con un límite p99 además del p95. El p99 es el tiempo que baten 99 de cada 100 órdenes, así que solo 1 de cada 100 puede tardar más. Un diseño con unas pocas órdenes muy atascadas puede superar el p95 y fallar aun así el p99.

Si los matchers se detienen, las órdenes que esperan en el stream siguen contando: cuando termina una prueba, cada orden aún sin casar cuenta con la antigüedad que tenga en ese momento.

## Las pruebas

Hay una prueba visible. Puedes ejecutarla tantas veces como quieras:

- **Una sesión normal**: el tráfico descrito arriba, en su pico.

Hay dos pruebas ocultas más, de las que solo ves el nombre: **Abre el mercado** y **Se cae un matcher**. Tu diseño tiene que superar las tres.
