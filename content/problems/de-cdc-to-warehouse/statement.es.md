# Pedidos hacia el almacén de datos

## El producto

Tienes una tienda online. Cada pedido vive en una base de datos PostgreSQL: realizado, pagado, empaquetado, enviado, devuelto. Los analistas quieren los mismos pedidos en el almacén de datos, Snowflake, para que sus paneles muestren las ventas de hoy y no las de ayer. La base de datos de la tienda no debe ir más lenta por ellos, y ningún cambio puede perderse por el camino.

Los analistas aceptan un almacén que vaya unos minutos por detrás, unos 5 minutos. Las pruebas del simulador duran de 60 a 90 segundos y sus percentiles llegan hasta un minuto más o menos, así que el problema está a escala: aquí un cambio debe llegar al almacén **en 15 s o menos** desde su confirmación, y la venta flash y la ralentización duran 30 s en lugar de la media hora que durarían en la vida real.

## Lo que debes cumplir

| Objetivo | Valor |
| --- | --- |
| Tráfico pico | 12 cambios de pedido/s |
| Frescura (p95, desde la confirmación hasta que el almacén tiene el cambio) | 15 s o menos (5 minutos en la vida real, a escala) |
| Disponibilidad | 99,9 % o más |
| Errores | 0,1 % o menos |
| Cambios perdidos | 0 |
| Presupuesto | 7900 $ al mes |

## El tráfico

- **Cambiar un pedido** (100 %). La petición puede pasar por un balanceador de carga, llega a tus App Servers y estos escriben el cambio en la base de datos SQL. Después entra en juego la captura de cambios (CDC): la base de datos publica cada cambio confirmado en un Stream, y un consumidor (un procesador de streams o un Worker) lo toma del Stream y lo escribe en el almacén de datos. Hay 1 000 000 de pedidos, y todos tienen más o menos la misma probabilidad de cambiar.

La latencia se mide hasta completar: desde que el usuario cambia el pedido hasta que el almacén tiene el cambio. La respuesta al usuario sale en la confirmación, así que la tienda nunca espera al almacén.

En el simulador, la captura de cambios es una conexión Publish que va directa de la base de datos SQL al Stream. Representa un conector como Debezium leyendo el registro de escritura anticipada (WAL) de PostgreSQL; no hace falta añadir una tarjeta Conector de datos aparte. Si el Stream no puede recibir un cambio, el cambio espera en el registro de la base de datos (su "Búfer de CDC", 300 s por defecto) en lugar de perderse.

El almacén escribe cada cambio por separado, y una escritura tarda unos 300 ms de mediana, con una cola lenta. El consumidor conserva su hueco durante toda la escritura.

## Las pruebas

Hay una prueba visible. Puedes ejecutarla tantas veces como quieras:

- **Un día normal**: el tráfico descrito arriba, en su pico.

Hay dos pruebas ocultas más, de las que solo ves el nombre: **Una venta flash: 3 veces los cambios durante 30 s** y **El almacén de datos se ralentiza: cada escritura tarda 1,5 s más durante 30 s**. Tu diseño tiene que superar las tres.
