# Solución comentada

## Adónde va la carga

La tabla guarda 365 días en 36,5 GB, así que un día son 0,1 GB. Todo depende de cuánto de la tabla tiene que leer una búsqueda para encontrar ese día.

Si la tabla no está particionada, el almacén no sabe dónde empieza un día, así que cada búsqueda lee los 36,5 GB. Eso lleva 0,3 s de sobrecarga + 36,5 s de escaneo + 292 archivos de 128 MB × 20 ms ÷ 8 ≈ 37,5 s, y BigQuery cobra 36,5 GB, unos 0,22 $, por cada búsqueda. Llegan dos búsquedas por segundo, así que habría unas 2 × 37,5 = 75 consultas ejecutándose a la vez, muchas más que cualquier número de huecos que le dieras.

Particionada por día, una búsqueda lee solo su día: 0,3 s + 0,1 s + un archivo ≈ 0,42 s, y 0,1 GB, unos 0,0006 $. Dos búsquedas por segundo mantienen entonces ocupado más o menos un hueco.

Las consultas del año entero son distintas: necesitan el año entero se organice como se organice la tabla, así que cada una sigue tardando unos 37,5 s y costando unos 0,22 $. En un día normal hay unas 0,02 por segundo, menos de un hueco ocupado de media. Las pruebas ocultas cambian eso. La auditoría lanza 15 veces más, 0,3 por segundo, que mantienen ocupados unos 11 huecos de media y más en las ráfagas. El incidente multiplica todo el tráfico por 10 durante 30 segundos: unos 8 huecos para las búsquedas más unos 7,5 para las consultas del año entero.

## El diseño de referencia

El Cliente de los ingenieros (Tiempo límite 60 000 ms) llama a 2 Servidores de aplicaciones Go, que consultan Google BigQuery con 1 instancia y Concurrencia por instancia 32. BigQuery lee una tabla Apache Iceberg por una conexión de Consulta, con Particionado Por día, Compactación Diaria (archivos de 128 MB) y Datos guardados (TB) 0,04.

Cuesta unos 19 124 $ al mes: 18 983 $ de BigQuery, 140 $ de los Servidores de aplicaciones y 1 $ de la tabla. De la factura de BigQuery, unos 3200 $ son las búsquedas de 24 horas y unos 15 800 $ las consultas del año entero. La ejecución normal sacó 8 en 5 minutos, más o menos 1 consulta de cada 75, y son la mayor parte de la factura. Particionar no puede ayudar a una consulta que de verdad necesita el año entero.

«Un día de trabajo normal» tiene un p95 de unos 417 ms y ningún error. «Una auditoría lanza quince veces las consultas del año entero de siempre» tiene un p95 de unos 37,4 s, que son las propias consultas del año entero, y ningún error: 32 huecos aguantan las consultas de la auditoría y dejan sitio para las búsquedas. «Un incidente: todo el mundo busca en el último día a la vez» tiene un p95 de unos 417 ms y ningún error.

El juez solo pone precio al día normal. Un mes con el tráfico de la auditoría costaría unos 173 000 $, y un mes con el tráfico del incidente unos 63 000 $: una auditoría que durase todo el mes reventaría el presupuesto, y las pruebas no lo comprueban. Trata las consultas del año entero como lo caras que son.

La compactación también importa. Sin ella (Compactación Ninguno), la tabla se queda con los archivos de 8 MB que escribe la ingesta en streaming. Una consulta del año entero abre entonces 4672 archivos en vez de 292, tarda unos 48 s, y la auditoría no cumple su p95 de 45 s (48,3 s). Las búsquedas apenas lo notan (451 ms).

Particionado Por hora da los mismos resultados aquí: una búsqueda de 24 horas lee lo mismo que un día de datos en ambos casos.

Con BigQuery bajo demanda, los huecos no cuestan nada aquí (cobra bytes), así que la referencia va sobrada. Con la Concurrencia por instancia por defecto del almacén, 8, el diseño supera el día normal pero falla la auditoría con un 11,4 % de errores (p95 de 58,9 s) y el incidente con un p95 de 28,9 s: los escaneos largos llenan los 8 huecos y las búsquedas esperan detrás. Con Concurrencia 16 lo supera todo (el p95 del incidente es de unos 671 ms); 32 deja margen.

En el reto de coste, el objetivo es el 50 % del coste de la referencia, y un diseño de prueba se queda en un 8 % más o menos (1526 $). Mantiene la misma forma y el mismo particionado, con un Servidor de aplicaciones y Snowflake en vez de BigQuery. Snowflake cobra las horas que funciona un almacén (un X-Small, 1460 $ al mes si está ocupado todo el mes), no los bytes que escanea, y con 2 consultas por segundo constantes todo el mes sale mucho más barato que pagar por TB. Eso sí, lee la advertencia de abajo antes de fiarte: aquí la velocidad de escaneo de un almacén no depende de su tamaño.

## Por qué falla el primer intento

Un primer intento habitual es el mismo diseño con la tabla en Particionado Ninguno. Falla todas las pruebas, la visible incluida.

En «Un día de trabajo normal», cada búsqueda lee el año entero y tarda unos 37,5 s. Los 32 huecos se llenan enseguida, las consultas hacen cola y a los 60 s el Cliente se rinde: falla el 90,1 % de las consultas, con un p95 de unos 60,1 s. El almacén está ocupado al 97 % y los Servidores de aplicaciones, esperándolo, al 87 %. Cada consulta que consiguió un hueco (más o menos la mitad) se cobró por el año entero que leyó: solo BigQuery sale por unos 562 000 $ al mes, 16 veces el presupuesto. La auditoría (90,8 % de errores) y el incidente (92,0 % de errores) fallan igual.

Más huecos no lo arreglarían: 75 consultas a la vez de 37,5 s cada una siguen sin cumplir el objetivo de 2 s, y la factura crece con cada consulta que se ejecuta. El arreglo es leer menos.

La idea es esta: un almacén cobra, y tarda, por lo que lee, no por lo que preguntas. Particiona la tabla por la columna que filtran tus consultas, aquí la marca de tiempo, para que una búsqueda de un día lea un día. Después dimensiona el almacén para las consultas que siguen leyéndolo todo.

## Lo que el simulador deja fuera

Una búsqueda real de las últimas 24 horas suele tocar dos particiones de día (ayer y hoy), así que lee hasta el doble; aquí lee un día. BigQuery bajo demanda ejecuta de verdad las consultas en un conjunto compartido de huecos con sus propios límites; aquí la Concurrencia por instancia es una propiedad que eliges, y no cuesta nada. La velocidad de escaneo de un almacén no depende aquí de su tamaño, y eso favorece a un almacén Snowflake pequeño: un X-Small real reparte sus pocos núcleos entre sus consultas, y las consultas del año entero tardarían mucho más. Las tablas por columnas solo leen las columnas que nombra la consulta; aquí el tamaño de una consulta es fijo. No se usa la caché de resultados de BigQuery: la tabla de logs cambia todo el rato, lo que invalida los resultados guardados, así que aquí ninguna consulta se repite. La escritura de los logs en la tabla no se simula. La factura mensual supone que el pico dura todo el mes; el tráfico real baja por la noche.

## Para probar

- Vuelve a poner Particionado en Ninguno y ejecuta el día normal. Mira los huecos del almacén y el coste de BigQuery.
- Pon Compactación en Ninguno y ejecuta la auditoría. Compara lo que tardan las consultas del año entero con su límite de p95.
- Pon la Concurrencia por instancia en 8 y ejecuta el incidente.
- Cambia BigQuery por Snowflake y compara los costes; luego vuelve a leer «Lo que el simulador deja fuera».
