# Solución comentada

## Adónde va el tiempo

Empieza por una consulta, sola en el almacén de datos. Su tiempo es la sobrecarga por consulta (300 ms), más los datos que lee a la velocidad de escaneo, más unos 20 ms por cada archivo que abre, 8 archivos a la vez.

Un día normal son 2 GB. Con la tabla particionada por día, la consulta lee solo hoy: 2 GB a 8 GB/s son 0,25 s. En archivos de 4 MB, esos 2 GB son 512 archivos, y abrirlos cuesta 512 × 20 ms ÷ 8 ≈ 1,28 s. Los archivos cuestan cinco veces más que la lectura. Compactado en archivos de 256 MB, el mismo día son 8 archivos: 0,02 s. Así que una consulta tarda unos 1,83 s con los archivos pequeños y 0,57 s compactado.

Un día de rebajas son 20 GB, y el pipeline los escribió en los mismos archivos de 4 MB: 5120 archivos, 12,8 s solo para abrirlos, y unos 15,6 s por consulta. Eso ya supera el límite de 8 s antes de que ninguna consulta espere a otra. Compactado, son 80 archivos y unos 3,0 s.

Ese es el centro del problema: el número de archivos pone un suelo bajo cada consulta, y ningún número de instancias del almacén lo baja. Las instancias solo deciden cuántas consultas se ejecutan a la vez. Eso importa en la revisión de fin de mes, cuando llegan cuatro veces las consultas: unas 33 por segundo.

## El diseño de referencia

Un AWS ALB envía el tráfico a 2 servidores de aplicaciones de Go. Los lotes de eventos van a un stream de Apache Kafka con 48 particiones, que consumen 4 instancias de Apache Flink (concurrencia 20) y escriben en una Tabla lakehouse (Delta Lake), particionada por día, con la Compactación en Cada hora y un tamaño de archivo objetivo de 256 MB. Los servidores de aplicaciones envían las consultas a un Almacén de datos (Databricks SQL) con 4 instancias, una concurrencia de 8, una velocidad de escaneo de 8 GB/s, la sobrecarga por consulta por defecto de 300 ms y la caché de resultados activada, que lee la tabla en su sitio. El Cliente espera hasta 60 000 ms.

Cuesta unos 4599 $ al mes: 4000 $ del almacén de datos, 280 $ del procesador de streams, 153 $ del stream, 140 $ de los servidores de aplicaciones, 25 $ del ALB y 1 $ del almacenamiento de la tabla. Un día normal tiene un p95 de unos 584 ms sin errores, y el almacén está ocupado alrededor del 12 % del tiempo.

"Un día de rebajas: diez veces los eventos, y los analistas miran el día de hoy" envía diez veces los lotes de eventos, y cada consulta lee los 20 GB del día de rebajas. El p95 es de unos 3022 ms sin errores ni eventos perdidos: cada consulta compactada tarda unos 3 s, y 32 huecos aguantan las unas 8 consultas por segundo con margen.

"Revisión de fin de mes: cuatro veces las consultas" mantiene el p95 en unos 584 ms. A unos 0,57 s por consulta, 33 consultas por segundo ocupan unos 15 de los 32 huecos, y la caché de resultados responde casi al instante una de cada cinco.

Para el reto de coste, la búsqueda no encontró ningún diseño por debajo del 80 % del coste de la referencia (el mejor llegó al 94 %), así que este problema no tiene reto. El almacén de datos es casi toda la factura, y con menos instancias queda poco margen para el día de rebajas.

## Por qué falla el primer intento

Un primer intento habitual es el mismo diseño con la Compactación en Ninguna: los archivos del procesador de streams se quedan como se escribieron, y el tamaño de archivo objetivo no hace nada. Cuesta los mismos 4599 $ y supera "Un día normal" con un p95 de unos 1842 ms, pero cada consulta ocupa un hueco tres veces más tiempo, y el almacén está ocupado alrededor del 39 % del tiempo.

En el día de rebajas, cada consulta tarda unos 15,6 s por sí sola. El p95 sube a unos 43 778 ms y falla alrededor del 8,9 % de las peticiones: las consultas se amontonan unas detrás de otras, y las que esperan más que el tiempo límite de 60 s del Cliente se abandonan. Añadir instancias no ayuda: el mismo diseño con 16 instancias del almacén (16 599 $, el doble del presupuesto) sigue con un p95 de unos 15 633 ms en el día de rebajas, porque eso es lo que tarda una consulta.

A fin de mes, cuatro veces las consultas a unos 1,8 s cada una necesitan unos 49 huecos, y hay 32. La cola crece durante toda la prueba y el p95 llega a unos 31 890 ms. Aquí más instancias sí ayudarían, pero costarían mucho más que compactar.

La idea es esta: la ingesta en streaming cambia tamaño de archivo por frescura, y cada lector lo paga en cada consulta. La compactación es el trabajo en segundo plano que lo devuelve una sola vez. Dimensiona el almacén para cuántas consultas se ejecutan a la vez, pero arregla primero el número de archivos, porque decide cuánto tarda cada una.

## Lo que el simulador deja fuera

Cada prueba dura 60 s y representa una hora de ese tráfico; la tabla no crece durante una ejecución. Aquí la compactación es gratis y completa: el trabajo que junta los archivos no cuesta nada ni usa tiempo del almacén ni del procesador de streams, y todos los archivos ya están compactados, mientras que una compactación real cada hora deja la hora más reciente en archivos pequeños hasta su siguiente ejecución. No se simulan las estadísticas de archivo de Delta Lake, el data skipping ni el Z-ordering, que permiten a una consulta saltarse archivos dentro de una partición. Los 20 ms por archivo y los 8 lectores en paralelo son decisiones del modelo, no mediciones. Databricks SQL solo factura los segundos en que está ocupado más su espera de auto-suspensión, pero este tráfico constante nunca le deja suspenderse, así que factura todas las horas. Los precios son los precios didácticos del registro.

## Para probar

- Pon la Compactación en Ninguna y ejecuta el día de rebajas con 4 y luego con 16 instancias del almacén. Mira cómo el p95 deja de bajar.
- Pon el tamaño de archivo objetivo en 32 MB. Calcula el número de archivos de un día de rebajas y compara el p95.
- Pon el Particionado en Ninguno. Cada consulta lee ahora los 60 GB en vez de un día: compara el p95 con el efecto de la compactación.
