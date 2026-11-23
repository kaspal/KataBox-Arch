# Solución comentada

## Adónde va la carga

Dos flujos comparten este diseño. Los lotes de clics llegan a 180 por segundo y pasan por el Stream y el Procesador de streams hasta la tabla. Los refrescos de gráficos llegan a 20 por segundo, y cada uno es un escaneo que ocupa un hueco del almacén hasta que termina.

El tiempo se va en el escaneo. La tabla guarda 730 GB, un año de clics. Sin particiones, un refresco los lee todos: a 4 Gbps son unos tres minutos. Particionada por día, lee solo el último día, unos 2 GB, que tardan 0,5 s más los 300 ms de sobrecarga del almacén. Luego vienen los archivos. Abrir cada uno cuesta 20 ms, ocho a la vez. En archivos de 2 MB, el día son 1024 archivos, que añaden 2,56 s. En archivos de 256 MB, son 8 archivos, unos 20 ms. Así que la misma respuesta de 2 GB tarda unos 3,4 s desde archivos pequeños y unos 0,82 s desde archivos compactados.

Ese tiempo es también lo que el refresco ocupa su hueco. A 20 refrescos por segundo, los archivos pequeños mantienen ocupados unos 67 huecos y los compactados unos 16. En un día de lanzamiento el tráfico es cinco veces mayor: unos 336 huecos frente a unos 82.

## El diseño de referencia

Un AWS ALB manda el tráfico a 6 Servidores de aplicaciones Go. Estos publican los lotes de clics en Apache Kafka con 96 particiones. Un Procesador de streams Apache Flink con 4 instancias y una concurrencia de 20 (80 huecos) los consume y los escribe en una Tabla lakehouse Apache Iceberg particionada por día, con compactación cada hora, un tamaño de archivo objetivo de 256 MB y 0,73 TB guardados. Los refrescos de gráficos van de los Servidores de aplicaciones a un Almacén de datos Snowflake con 4 instancias, una concurrencia de 32 (128 huecos) y una velocidad de escaneo de 4 Gbps, que lee la tabla por una arista Consulta. Esto cuesta unos 6735 $ al mes: 5840 $ de Snowflake, 420 $ de los Servidores de aplicaciones, 280 $ de Flink, 153 $ de Kafka, 25 $ del ALB y 17 $ del almacenamiento de la tabla. Un día normal tiene un p95 de unos 834 ms, sin errores y sin lotes perdidos. El almacén está ocupado en torno al 13 % y Flink en torno al 14 %.

«Un día de lanzamiento trae 5× el tráfico» multiplica todo el tráfico por cinco desde el segundo 20, durante 20 segundos: 900 lotes y 100 refrescos por segundo. Cada refresco sigue tardando unos 0,82 s, así que están ocupados unos 82 de los 128 huecos del almacén, y nada hace cola. Flink necesita unos 56 de sus 80 huecos, y las 96 particiones de Kafka dejan que trabajen todos a la vez. El p95 se queda en unos 834 ms, sin errores.

«El procesador de streams cae durante 20 s» tumba Flink en el segundo 20, durante 20 segundos, en una ejecución de 90 segundos. Esta prueba admite un p95 de 30 s, porque los clics pueden llegar tarde, pero no se puede perder ninguno. Los Servidores de aplicaciones siguen publicando, y Kafka guarda los 3600 lotes que llegan mientras tanto (su retención es de 168 horas). Cuando Flink vuelve, sus huecos de sobra vacían el atraso en unos segundos. Los lotes que esperaron suben el p95 a unos 15,6 s, más o menos la mitad del límite, y no se pierde ninguno. La ejecución también informa de 3591 duplicados: los lotes que Flink procesó desde su último checkpoint se procesan otra vez al reiniciar. Este problema no juzga los duplicados (mira más abajo).

La búsqueda del reto de coste no encontró ningún diseño al 80 % de este coste o menos (lo mejor fue el 95 %), así que este problema no tiene reto de coste.

## Por qué falla el primer intento

Un primer intento habitual es el mismo diseño, pero la tabla se queda con los archivos que escribe Flink: la compactación está apagada, así que cada archivo tiene unos 2 MB. Cuesta lo mismo, 6735 $, porque el simulador no cobra la compactación. Supera «Un día normal» con un p95 de unos 3,4 s, por debajo del límite de 4 s: cada refresco abre 1024 archivos, y el almacén está ocupado en torno al 52 %.

En el día de lanzamiento, 100 refrescos por segundo ocupan cada uno un hueco durante unos 3,4 s, lo que pide unos 336 huecos. El almacén tiene 128. Termina unos 38 refrescos por segundo, así que el resto hace cola, y la cola crece unos 60 por segundo. Los refrescos que esperan demasiado fallan: falla en torno al 7,4 % de todas las peticiones, unas tres cuartas partes de los refrescos de gráficos de la prueba. Los lotes de clics ni se enteran, porque no tocan el almacén.

Comprar más instancias del almacén es la otra salida, a 1460 $ cada una. Con 8, el día de lanzamiento sigue fallando; con 10 pasa, pero el diseño cuesta unos 15 495 $, por encima del presupuesto de 12 000 $. La compactación, en cambio, hace cada escaneo cuatro veces más corto, y aquí sin coste. La idea es esta: quien escribe desde un stream crea muchos archivos pequeños, y cada consulta paga por abrir cada uno. Compáctalos, particiona para que una consulta lea solo lo que necesita, y el mismo almacén responde muchas más consultas.

## Lo que el simulador deja fuera

Aquí la compactación es gratis e instantánea: una tabla compactada se trata como si solo tuviera archivos del tamaño objetivo. Un trabajo de compactación real gasta cómputo, se ejecuta según un calendario y deja pequeños los archivos más nuevos hasta que se ejecuta. Con compactación diaria, los archivos de hoy seguirían siendo pequeños todo el día, y una consulta sobre el último día los seguiría abriendo; por eso la referencia compacta cada hora. Snowflake cobra en créditos según el tamaño del almacén; aquí una instancia cuesta 1460 $ al mes mientras está ocupada, y los precios son las cifras para aprender del registro, no presupuestos. Un sink real de Flink a Iceberg confirma sus archivos en cada checkpoint, así que un reinicio reescribe los archivos sin confirmar en vez de añadir filas dos veces; el simulador cuenta las repeticiones como duplicados, y este problema no los juzga. Aquí los clics se ven en las consultas en cuanto se escriben, no en la siguiente confirmación. No se simulan la planificación de consultas, las lecturas de metadatos, el desequilibrio de datos entre clientes ni el coste por petición del almacenamiento de objetos.

## Para probar

- Pon la compactación en Ninguno y ejecuta el día de lanzamiento. Luego añade instancias al almacén hasta que pase, y compara el coste.
- Pon el particionado en Ninguno y ejecuta un día normal. Cuenta cuánto de la tabla escanea un refresco.
- Baja Kafka a 12 particiones y ejecuta el día de lanzamiento. Mira cuánto tardan en llegar los lotes de clics.
