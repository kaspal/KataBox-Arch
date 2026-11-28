# Solución comentada

## Adónde va el tiempo

Empieza por una consulta, sola en el almacén de datos. Su tiempo es la sobrecarga por consulta (300 ms), más los datos que lee a la velocidad de escaneo, más unos 20 ms por cada archivo que abre, 8 archivos a la vez.

Un día normal son 250 MB. Con la tabla particionada por día, la consulta lee solo hoy: 250 MB a 5 GB/s son 0,05 s. En archivos de 500 KB, esos 250 MB son 512 archivos, y abrirlos cuesta 512 × 20 ms ÷ 8 ≈ 1,28 s. Los archivos cuestan unas veinticinco veces lo que la lectura. Compactado en archivos de 256 MB, el mismo día es un solo archivo: unos pocos milisegundos. Así que una consulta tarda unos 1,63 s con los archivos pequeños y 0,35 s compactado.

Un día de rebajas son 2,5 GB, y el pipeline los escribió en los mismos archivos de 500 KB: 5120 archivos, 12,8 s solo para abrirlos, y unos 13,6 s por consulta. Eso ya supera el límite de 8 s antes de que ninguna consulta espere a otra. Compactado, son 10 archivos y unos 0,83 s.

Ese es el centro del problema: el número de archivos pone un suelo bajo cada consulta, y ningún número de instancias del almacén lo baja. Las instancias deciden cuántas consultas se ejecutan a la vez, y cuánto leen entre todas: las instancias de un almacén se reparten su velocidad de escaneo entre las consultas que corren a la vez. Los huecos importan en la revisión de fin de mes, cuando llegan cuatro veces las consultas: unas 33 por segundo. La velocidad de escaneo total importa en el día de rebajas, cuando cada consulta lee diez veces más.

## El diseño de referencia

Un AWS ALB envía el tráfico a 2 servidores de aplicaciones de Go. Los lotes de eventos van a un stream de Apache Kafka con 48 particiones, que consumen 2 instancias de Apache Flink (la concurrencia por defecto de 4) y escriben en una Tabla lakehouse (Delta Lake), particionada por día, con la Compactación en Cada hora y un tamaño de archivo objetivo de 256 MB. Los servidores de aplicaciones envían las consultas a un Almacén de datos (Databricks SQL) con 4 instancias, una concurrencia de 8, una velocidad de escaneo de 5 GB/s, la sobrecarga por consulta por defecto de 300 ms y la caché de resultados activada, que lee la tabla en su sitio. El Cliente espera hasta 60 000 ms.

Cuesta unos 41 338 $ al mes: 40 880 $ del almacén de datos, 153 $ del stream, 140 $ del procesador de streams, 140 $ de los servidores de aplicaciones, 25 $ del ALB y menos de 1 $ del almacenamiento de la tabla. Un día normal tiene un p95 de unos 370 ms sin errores, y el almacén está ocupado alrededor del 7 % del tiempo.

La velocidad de escaneo es el tamaño del almacén, y se paga: una instancia a 1 GB/s, el valor por defecto, cuesta 2044 $ al mes, y una a 5 GB/s cinco veces eso, 10 220 $. La marca el día de rebajas: unas 8 consultas por segundo, una de cada cinco respondida por la caché de resultados, leen 2,5 GB cada una, unos 16,7 GB por segundo en total. Las 4 instancias a 5 GB/s leen 20 GB por segundo entre todas. Con 4 instancias a 4 GB/s (16 GB por segundo, 33 162 $ en total) la cola crece durante el día de rebajas y el p95 llega a unos 3,4 s; con 3 a 5 GB/s (31 118 $), a unos 5,5 s. Las dos pasan, pero con poco margen; 5 instancias a 4 GB/s cuestan lo mismo que la referencia y llegan a unos 1,0 s.

"Un día de rebajas: diez veces los eventos, y los analistas miran el día de hoy" envía diez veces los lotes de eventos, y cada consulta lee los 2,5 GB del día de rebajas. El p95 es de unos 868 ms sin errores ni eventos perdidos: cada consulta compactada tarda unos 0,83 s, y las consultas leen unos 16,7 de los 20 GB por segundo del almacén.

"Revisión de fin de mes: cuatro veces las consultas" mantiene el p95 en unos 370 ms. A unos 0,35 s por consulta, 33 consultas por segundo ocupan unos 10 de los 32 huecos, y la caché de resultados responde casi al instante una de cada cinco.

Para el reto de coste, un diseño de prueba sale por un 10 % del coste de la referencia (4068 $), por debajo del objetivo del 50 %. Mantiene la tabla compactada, pero usa 1 servidor de aplicaciones, 1 instancia de Flink y 3 nodos de ClickHouse a 5 GB/s (3750 $) en lugar de Databricks SQL. En el día de rebajas sus 15 GB por segundo se quedan cortos y el p95 llega a unos 5,5 s.

## Por qué falla el primer intento

Un primer intento habitual es el mismo diseño con la Compactación en Ninguna: los archivos del procesador de streams se quedan como se escribieron, y el tamaño de archivo objetivo no hace nada. Cuesta los mismos 41 338 $ y supera "Un día normal" con un p95 de unos 1635 ms, pero cada consulta ocupa un hueco casi cinco veces más tiempo, y el almacén está ocupado alrededor del 35 % del tiempo.

En el día de rebajas, cada consulta tarda unos 13,6 s por sí sola. El p95 sube a unos 50 287 ms y falla alrededor del 2,6 % de las peticiones: las consultas se amontonan unas detrás de otras, y las que esperan más que el tiempo límite de 60 s del Cliente se abandonan. Añadir instancias no ayuda: el mismo diseño con 16 instancias del almacén (163 978 $, más del doble del presupuesto) sigue con un p95 de unos 13 610 ms en el día de rebajas, porque eso es lo que tarda una consulta.

A fin de mes, cuatro veces las consultas a unos 1,6 s cada una necesitan unos 44 huecos, y hay 32. La cola crece durante toda la prueba y el p95 llega a unos 23 695 ms. Aquí más instancias sí ayudarían, pero costarían mucho más que compactar.

La idea es esta: la ingesta en streaming cambia tamaño de archivo por frescura, y cada lector lo paga en cada consulta. La compactación es el trabajo en segundo plano que lo devuelve una sola vez. Dimensiona el almacén para cuántas consultas se ejecutan a la vez, pero arregla primero el número de archivos, porque decide cuánto tarda cada una.

## Lo que el simulador deja fuera

Cada prueba dura 60 s y representa una hora de ese tráfico; la tabla no crece durante una ejecución. Aquí la compactación es gratis y completa: el trabajo que junta los archivos no cuesta nada ni usa tiempo del almacén ni del procesador de streams, y todos los archivos ya están compactados, mientras que una compactación real cada hora deja la hora más reciente en archivos pequeños hasta su siguiente ejecución. No se simulan las estadísticas de archivo de Delta Lake, el data skipping ni el Z-ordering, que permiten a una consulta saltarse archivos dentro de una partición. Los 20 ms por archivo y los 8 lectores en paralelo son decisiones del modelo, no mediciones. Databricks SQL solo factura los segundos en que está ocupado más su espera de auto-suspensión, pero este tráfico constante nunca le deja suspenderse, así que factura todas las horas; un escaneo más rápido es un tamaño mayor, y se cobra en proporción. Los precios son los precios didácticos del registro.

## Para probar

- Pon la Compactación en Ninguna y ejecuta el día de rebajas con 4 y luego con 16 instancias del almacén. Mira cómo el p95 deja de bajar.
- Pon el tamaño de archivo objetivo en 32 MB. Calcula el número de archivos de un día de rebajas y compara el p95.
- Pon el Particionado en Ninguno. Cada consulta lee ahora los 7,5 GB en vez de un día: compara el p95 con el efecto de la compactación.
