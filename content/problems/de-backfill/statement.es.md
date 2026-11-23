# Un relleno junto al stream en vivo

## El producto

Llevas el pipeline de eventos de una empresa de analítica de producto. Las apps envían los clics y las páginas vistas de sus usuarios a tu API de ingesta, que los agrupa en lotes de 500 eventos y publica cada lote en un Stream. Un Procesador de streams lee los lotes y carga cada uno en un Almacén de datos, donde los leen los paneles de los clientes. Los clientes esperan ver un clic en su panel en cuestión de segundos.

Esta semana el equipo encontró un error que perdía un campo en tres meses de eventos, así que hay que volver a cargar el histórico corregido. El relleno lee los eventos antiguos del archivo y los envía por la misma API de ingesta, el mismo Stream, el mismo Procesador de streams y el mismo almacén que los eventos en vivo: un solo pipeline, un solo juego de tablas. Está ajustado para usar el hueco que deja el tráfico en vivo. Nadie está mirando el relleno; solo tiene que aceptarse y cargarse tarde o temprano, sin perder ningún lote.

## Lo que debes cumplir

| Objetivo | Valor |
| --- | --- |
| Tráfico pico | 40 lotes/s (24 en vivo, 16 de relleno) |
| Latencia p95 (ver abajo) | 10 s o menos |
| Disponibilidad | 99,9 % o más |
| Errores | 0,1 % o menos |
| Lotes perdidos | ninguno |
| Presupuesto | 7500 $ al mes |

## El tráfico

- **Cargar un lote de eventos en vivo** (60 %). La petición puede pasar por un balanceador de carga y llega a tus Servidores de aplicaciones (la API de ingesta), que publican el lote en el Stream; el consumidor del Stream lo carga en el Almacén de datos. Un lote en vivo se mide por **finalización**: desde que la API lo recibe hasta que está cargado en el almacén. Eso es lo al día que van los paneles.
- **Cargar un lote de eventos históricos (relleno)** (40 %). La misma ruta. Un lote del relleno se mide por **respuesta**: desde que la API lo recibe hasta que el Stream lo ha guardado y la API contesta. Una vez en el Stream, el relleno puede esperar.

Así que el objetivo de p95 se aplica a la finalización de los lotes en vivo y a la confirmación de los lotes del relleno.

Un Almacén de datos está hecho para lecturas grandes, no para escrituras pequeñas: cargar un lote ocupa un hueco del almacén unos 300 ms (más en la cola lenta), y un almacén Snowflake tiene 8 huecos por instancia. El Procesador de streams conserva su hueco mientras dura la carga, y el Stream entrega un lote por partición a la vez.

## Las pruebas

Hay una prueba visible. Puedes ejecutarla tantas veces como quieras:

- **Una hora punta normal, con el relleno a su ritmo**: el tráfico descrito arriba, en su pico.

Hay dos pruebas ocultas más, de las que solo ves el nombre: **El relleno corre a 5 veces su ritmo, en el pico** y **Un pico en vivo: 2,5 veces los eventos en vivo**. El relleno entero dura días; una prueba ve un minuto de él. Tu diseño tiene que superar las tres.
