# Sincronización de archivos en la nube

## El producto

Tienes un servicio de sincronización de archivos. La gente guarda carpetas en sus móviles y portátiles, y cuando un archivo cambia, el dispositivo lo corta en bloques y te envía los bloques que han cambiado. Tú guardas cada bloque, llevas un registro de qué bloques forman cada archivo y avisas a los demás dispositivos del usuario de que algo ha cambiado para que lo descarguen. Muchos bloques te llegan más de una vez: el mismo archivo está en varios dispositivos, y una carpeta compartida guarda los mismos archivos para mucha gente. Un cambio que ha enviado un dispositivo nunca debe perderse.

## Lo que debes cumplir

| Objetivo | Valor |
| --- | --- |
| Tráfico pico | 2000 peticiones/s |
| Latencia p95 hasta completar | 5 s o menos |
| Disponibilidad | 99,9 % o más |
| Presupuesto | 6000 $ al mes |
| Mensajes perdidos | Ninguno |

## El tráfico

Los dispositivos llegan a la Sync API, un App Server, normalmente a través de un Load Balancer.

- **Subir un bloque modificado** (85 %). Cada bloque ocupa 128 KB. Si tu diseño tiene una Cache delante, la Sync API busca primero el bloque en ella: un acierto significa que el servicio ya tiene ese bloque, y la subida termina ahí. Si no, el bloque se guarda en el almacenamiento de objetos, el registro del archivo se escribe en la base de datos y se publica un aviso de cambio en una cola, de donde lo toma un Worker que avisa a los demás dispositivos del usuario. La latencia se mide hasta que los demás dispositivos quedan avisados, no hasta que responde la Sync API.
- **Listar una carpeta** (15 %). La Sync API lee de la base de datos los registros de la carpeta.

Hay 2 000 000 de bloques distintos, y la frecuencia con que se sube cada uno sigue una curva Zipf 1,2: un grupo bastante pequeño de bloques se lleva la mayoría de las subidas. No interviene ningún proveedor externo. Ten en cuenta que el almacenamiento de objetos y las colas se cobran por petición.

## Las pruebas

Hay una prueba visible. Puedes ejecutarla tantas veces como quieras:

- **Un día normal**: el tráfico descrito arriba, en su pico.

Hay dos pruebas ocultas más, de las que solo ves el nombre: **Un portátil nuevo lo sincroniza todo** y **El almacenamiento de objetos se ralentiza**. Tu diseño tiene que superar las tres.
