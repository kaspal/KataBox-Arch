# Solución comentada

## Adónde va la carga

Dos flujos comparten este diseño. Los lotes de clics llegan a 180 por segundo y pasan por el Stream y el Procesador de streams hasta la tabla. Los refrescos de gráficos llegan a 20 por segundo, y cada uno es un escaneo que ocupa un hueco del almacén hasta que termina.

El tiempo se va en el escaneo. La tabla guarda 730 GB, un año de clics. Sin particiones, un refresco los lee todos: a 2 Gbps son unos seis minutos. Particionada por día, lee solo el último día, unos 2 GB, que tardan 1 s más los 300 ms de sobrecarga del almacén. Luego vienen los archivos. Abrir cada uno cuesta 20 ms, ocho a la vez. En archivos de 2 MB, el día son 1024 archivos, que añaden 2,56 s. En archivos de 256 MB, son 8 archivos, unos 20 ms. Así que la misma respuesta de 2 GB tarda unos 3,9 s desde archivos pequeños y unos 1,3 s desde archivos compactados.

Ese tiempo es también lo que el refresco ocupa su hueco. A 20 refrescos por segundo, los archivos pequeños mantienen ocupados unos 77 huecos y los compactados unos 26. En un día de lanzamiento el tráfico es cinco veces mayor: unos 386 huecos frente a unos 132.

## El diseño de referencia

Un AWS ALB manda el tráfico a 6 Servidores de aplicaciones Go. Estos publican los lotes de clics en Apache Kafka con 96 particiones. Un Procesador de streams Apache Flink con 20 instancias y la concurrencia por defecto de 4 (80 huecos) los consume y los escribe en una Tabla lakehouse Apache Iceberg particionada por día, con compactación cada hora, un tamaño de archivo objetivo de 256 MB y 0,73 TB guardados. Los refrescos de gráficos van de los Servidores de aplicaciones a un Almacén de datos Snowflake con 5 instancias, una concurrencia de 32 (160 huecos) y una velocidad de escaneo de 2 Gbps, que lee la tabla por una arista Consulta. Esto cuesta unos 16 615 $ al mes: 14 600 $ de Snowflake, 1400 $ de Flink, 420 $ de los Servidores de aplicaciones, 153 $ de Kafka, 25 $ del ALB y 17 $ del almacenamiento de la tabla. Un día normal tiene un p95 de unos 1,34 s, sin errores y sin lotes perdidos. El almacén está ocupado en torno al 17 % y Flink en torno al 14 %.

La velocidad de escaneo es el tamaño del almacén, y se paga: una instancia a 1 Gbps, el valor por defecto, cuesta 1460 $ al mes, y una a 2 Gbps el doble, 2920 $. Un escaneo más rápido acorta solo la lectura, no la sobrecarga ni la apertura de archivos. A 1 Gbps un refresco compactado tarda unos 2,3 s, y 8 instancias (11 680 $) aguantan el día de lanzamiento; a 2 Gbps lo hacen 4 instancias por el mismo precio, con el día de lanzamiento justo por encima de sus 128 huecos. La referencia pone una quinta instancia a 2 Gbps, para que el día de lanzamiento tenga margen. Flink se cobra igual: una instancia tiene 4 huecos, y más huecos por instancia cuestan más en proporción, así que 80 huecos cuestan 1400 $ los repartas como los repartas.

«Un día de lanzamiento trae 5× el tráfico» multiplica todo el tráfico por cinco desde el segundo 20, durante 20 segundos: 900 lotes y 100 refrescos por segundo. Cada refresco sigue tardando unos 1,3 s, así que están ocupados unos 132 de los 160 huecos del almacén, y nada hace cola. Flink necesita unos 56 de sus 80 huecos, y las 96 particiones de Kafka dejan que trabajen todos a la vez. El p95 se queda en unos 1,34 s, sin errores.

«El procesador de streams cae durante 20 s» tumba Flink en el segundo 20, durante 20 segundos, en una ejecución de 90 segundos. Esta prueba admite un p95 de 30 s, porque los clics pueden llegar tarde, pero no se puede perder ninguno. Los Servidores de aplicaciones siguen publicando, y Kafka guarda los 3600 lotes que llegan mientras tanto (su retención es de 168 horas). Cuando Flink vuelve, sus huecos de sobra vacían el atraso en unos segundos. Los lotes que esperaron suben el p95 a unos 15,6 s, más o menos la mitad del límite, y no se pierde ninguno. La ejecución también informa de 3591 duplicados: los lotes que Flink procesó desde su último checkpoint se procesan otra vez al reiniciar. Este problema no juzga los duplicados. Para evitarlos, activa Escrituras idempotentes (dedup) en la tabla: las escrituras se fusionan por clave, las repeticiones no añaden filas y la ejecución no informa de duplicados, aquí sin coste.

La búsqueda del reto de coste no encontró ningún diseño al 80 % de este coste o menos (lo mejor fue el 98 %), así que este problema no tiene reto de coste.

## Por qué falla el primer intento

Un primer intento habitual es el mismo diseño, pero la tabla se queda con los archivos que escribe Flink: la compactación está apagada, así que cada archivo tiene unos 2 MB. Cuesta lo mismo, 16 615 $, porque el simulador no cobra la compactación. Supera «Un día normal» con un p95 de unos 3,8 s, justo por debajo del límite de 4 s: cada refresco abre 1024 archivos, y el almacén está ocupado en torno al 48 %.

En el día de lanzamiento, 100 refrescos por segundo ocupan cada uno un hueco durante unos 3,9 s, lo que pide unos 386 huecos. El almacén tiene 160. Termina unos 41 refrescos por segundo, así que el resto hace cola, y la cola crece unos 59 por segundo. Los refrescos que esperan demasiado fallan: falla en torno al 7,4 % de todas las peticiones, unas tres cuartas partes de los refrescos de gráficos de la prueba. Los lotes de clics ni se enteran, porque no tocan el almacén.

Comprar más instancias del almacén es la otra salida, a 2920 $ cada una. Con 10, el día de lanzamiento sigue fallando (un 5,2 % de errores); con 12 pasa, pero el diseño cuesta unos 37 055 $, por encima del presupuesto de 30 000 $. La compactación, en cambio, hace cada escaneo unas tres veces más corto, y aquí sin coste. La idea es esta: quien escribe desde un stream crea muchos archivos pequeños, y cada consulta paga por abrir cada uno. Compáctalos, particiona para que una consulta lea solo lo que necesita, y el mismo almacén responde muchas más consultas.

## Lo que el simulador deja fuera

Aquí la compactación es gratis e instantánea: una tabla compactada se trata como si solo tuviera archivos del tamaño objetivo. Un trabajo de compactación real gasta cómputo, se ejecuta según un calendario y deja pequeños los archivos más nuevos hasta que se ejecuta. Con compactación diaria, los archivos de hoy seguirían siendo pequeños todo el día, y una consulta sobre el último día los seguiría abriendo; por eso la referencia compacta cada hora. Snowflake cobra en créditos según el tamaño del almacén; aquí una instancia cuesta 1460 $ al mes mientras está ocupada a la velocidad de escaneo por defecto, más en proporción con un escaneo más rápido, y los precios son las cifras para aprender del registro, no presupuestos. Un sink real de Flink a Iceberg confirma sus archivos en cada checkpoint, así que un reinicio reescribe los archivos sin confirmar en vez de añadir filas dos veces; el simulador cuenta las repeticiones como duplicados salvo que la tabla tenga activadas las Escrituras idempotentes (dedup), y este problema no los juzga. Aquí los clics se ven en las consultas en cuanto se escriben, no en la siguiente confirmación. No se simulan la planificación de consultas, las lecturas de metadatos, el desequilibrio de datos entre clientes ni el coste por petición del almacenamiento de objetos.

## Para probar

- Pon la compactación en Ninguno y ejecuta el día de lanzamiento. Luego añade instancias al almacén hasta que pase, y compara el coste.
- Pon el particionado en Ninguno y ejecuta un día normal. Cuenta cuánto de la tabla escanea un refresco.
- Baja Kafka a 12 particiones y ejecuta el día de lanzamiento. Mira cuánto tardan en llegar los lotes de clics.
