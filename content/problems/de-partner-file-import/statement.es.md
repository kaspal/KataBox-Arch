# Importación de ficheros de socios

## El producto

Tu empresa recibe datos de cientos de socios. El servidor de cada socio sube ficheros a tu API de subida: listas de precios, pedidos, recuentos de stock. La API guarda el fichero en un almacenamiento de objetos, añade un mensaje para él a una cola y responde enseguida. Unos workers toman los mensajes, vuelven a leer el fichero del almacenamiento de objetos, lo analizan y cargan sus filas en PostgreSQL. Los socios también consultan sus importaciones: una página de estado lee PostgreSQL. Un fichero que nunca se carga son pedidos de un socio que nunca se envían, y nadie se entera hasta que el socio llama.

## Lo que debes cumplir

| Objetivo | Valor |
| --- | --- |
| Tráfico pico | 50 peticiones/s |
| Latencia p95 | 30 s o menos, desde la subida hasta que las filas están en PostgreSQL |
| Disponibilidad | 99,9 % o más |
| Errores | 0,1 % o menos |
| Ficheros perdidos | ninguno |
| Ficheros en la Cola de mensajes fallidos | ninguno |
| Presupuesto | 4200 $ al mes |

## El tráfico

- **Subir un fichero** (78 %). El servidor del socio llama a tus App Servers, que escriben el fichero (256 KB) en el almacenamiento de objetos y publican un mensaje en una cola. Un Worker lo consume, lee el fichero del almacenamiento de objetos y escribe sus filas en PostgreSQL. Su p95 se mide hasta que termina esa escritura. Analizar un fichero normal lleva el tiempo de servicio por defecto del Worker, 100 ms.
- **Subir un fichero gigante** (2 %). El mismo camino, pero el fichero supone 25 veces el trabajo en cada paso después de la cola: analizarlo, volver a leerlo y escribir sus filas.
- **Consultar el estado de una importación** (20 %). El servidor del socio llama a tus App Servers, que leen la fila de la importación en PostgreSQL.

Cuando falla un paso de un Worker, la cola vuelve a intentar el mensaje tras 1, 2, 4, 8, 16 y luego 32 segundos, hasta sus «Reintentos máximos» (3 por defecto: 7 segundos de espera en total). Un mensaje que falla en todos los reintentos va a una Cola de mensajes fallidos si hay una conectada a la cola; si no, se pierde. Un fichero en una Cola de mensajes fallidos se guarda, pero no se carga: alguien tiene que reprocesarlo a mano. El juez cuenta las dos cosas: no se puede perder ningún fichero, y ninguno puede acabar en la Cola de mensajes fallidos. Busca reintentos que duren más que un fallo por sí solos, para que la Cola de mensajes fallidos se quede vacía.

Una tarjeta PostgreSQL conectada con Replicar a otra guarda una copia. Si la primaria cae, la réplica sigue sirviendo lecturas, y se promueve para aceptar escrituras tras el «Tiempo de failover» de la primaria (30 s por defecto). Una PostgreSQL sola que cae se lleva con ella todas las lecturas y escrituras.

## Las pruebas

Hay una prueba visible. Puedes ejecutarla tantas veces como quieras:

- **Un día normal**: el tráfico descrito arriba, en su pico.

Hay dos pruebas ocultas más, de las que solo ves el nombre: **Un socio envía un mes de ficheros gigantes** y **Se cae la primaria de la base de datos**. Tu diseño tiene que superar las tres.
