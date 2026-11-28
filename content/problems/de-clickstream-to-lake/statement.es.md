# Clickstream hacia un lakehouse

## El producto

Tienes un servicio de analítica de producto. Tus clientes ponen un pequeño script en sus webs, y cada navegador te envía sus clics en lotes de unos 20. Guardas todos los clics en una tabla del lago sobre almacenamiento de objetos, y tus clientes miran gráficos de su tráfico: cada vez que un gráfico se refresca, lanza una consulta sobre la última hora de clics. La tabla guarda un año de clics, unos 730 GB de archivos columnares comprimidos (unos 2 GB al día, unos 85 MB por hora). Los clics pueden llegar a la tabla con unos segundos de retraso, pero no se puede perder ninguno, y un gráfico no se puede quedar colgado.

## Lo que debes cumplir

| Objetivo | Valor |
| --- | --- |
| Tráfico pico | 200 peticiones/s |
| Latencia p95 | 4 s o menos |
| Disponibilidad | 99,9 % o más |
| Errores | 1 % o menos |
| Lotes de clics perdidos | 0 |
| Presupuesto | 30 000 $ al mes |

El p95 cuenta todas las peticiones: un lote de clics hasta que está escrito en la tabla, y el refresco de un gráfico hasta que recibe su respuesta.

## El tráfico

- **Enviar un lote de eventos de clic** (90 %). La petición puede pasar por un balanceador de carga y llega a tus Servidores de aplicaciones, que la publican en un Stream. Un Procesador de streams la consume y la escribe en la Tabla lakehouse. El lote cuenta como hecho cuando está en la tabla.
- **Refrescar un gráfico del panel** (10 %). La petición puede pasar por un balanceador de carga y llega a tus Servidores de aplicaciones, que mandan la consulta a un Almacén de datos. El almacén lee la Tabla lakehouse donde está (conéctalo a la tabla con una arista Consulta) y escanea la última hora de clics. La tabla cambia cada segundo, así que el mismo gráfico nunca recibe una respuesta de caché.

Lo que tarda un escaneo depende de la tabla:

- **Particionado.** Sin particiones, una consulta lee el año entero. Por día, lee el día entero que necesita; por hora, solo la hora.
- **Archivos.** El Procesador de streams confirma archivos pequeños, de unos 50 KB cada uno. Sin compactación, la tabla se queda con ellos. Con compactación, se reescriben en archivos del tamaño de archivo objetivo de la tabla. Cada archivo le cuesta a la consulta un tiempo para abrirlo, sea del tamaño que sea.
- **Velocidad de escaneo.** Las instancias de un almacén se reparten su velocidad de escaneo entre las consultas que corren a la vez.

Un panel se rinde a los 5 s (el Tiempo límite por defecto del Cliente), y ese refresco cuenta como fallido. Un Servidor de aplicaciones conserva su hueco mientras espera al almacén.

## Las pruebas

Hay una prueba visible. Puedes ejecutarla tantas veces como quieras:

- **Un día normal**: el tráfico descrito arriba, en su pico.

Hay dos pruebas ocultas más, de las que solo ves el nombre: **Un día de lanzamiento trae 5× el tráfico** y **El procesador de streams cae durante 20 s**. Tu diseño tiene que superar las tres.
