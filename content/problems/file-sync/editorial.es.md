# Solución comentada

## Adónde va la carga

Con 2000 peticiones/s, unas 1700 son subidas de bloques y unas 300 son listados de carpetas. 1700 bloques de 128 KB son unos 218 MB de subidas cada segundo.

Sin ninguna comprobación, cada uno de esos bloques es una escritura en S3, una escritura en PostgreSQL y un mensaje en SQS. S3 se cobra a 1 $ por millón de peticiones y SQS a 0,40 $ por millón. 1700 por segundo durante un mes (2 592 000 s) son unos 4400 millones de peticiones en cada uno: unos 4400 $ para S3 y 1800 $ para SQS.

Con Zipf 1,2, la mayoría de las subidas repiten un bloque que ya tienes. Un índice de los bloques que tienes es pequeño: a unos 100 bytes por entrada, los 2 000 000 caben en unos 200 MB.

## El diseño de referencia

Un Load Balancer (AWS ALB) envía los dispositivos a 6 instancias Go de la Sync API. En cada subida, la API consulta primero el índice de bloques, una Cache Redis de 1 GB con un TTL de 86 400 s, con el patrón cache-aside. Un acierto significa que el bloque ya está guardado: en un día normal, cerca del 96 % de las subidas terminan ahí. Un fallo guarda el bloque en S3, escribe su registro en PostgreSQL y publica un aviso de cambio en una Queue SQS, que consumen 4 Notifiers Celery (concurrencia 32, 5 ms por aviso). Los listados de carpetas leen PostgreSQL, que tiene una réplica para repartir las lecturas. Cuesta unos 1706 $ al mes, de los que S3 son unos 159 $ y SQS unos 104 $. El p95 ronda los 26 ms, porque menos de 1 petición de cada 20 pasa del índice.

«Un portátil nuevo lo sincroniza todo» duplica las subidas de bloques y las hace mucho menos repetitivas (Zipf 0,7), con un límite de p95 de 1 s. Muchos más bloques fallan en el índice y recorren el camino completo: el p95 sube a unos 79 ms, y la factura a ese ritmo a unos 2586 $. «El almacenamiento de objetos se ralentiza» añade 150 ms a cada llamada a S3 durante 30 segundos, a partir del segundo 15, también con un límite de 1 s. Las pocas subidas que llegan a S3 tardan 150 ms más, pero son menos del 5 % del tráfico, así que el p95 se queda en unos 26 ms.

## Por qué falla el primer intento

Un primer intento habitual es el mismo diseño sin el índice de bloques: cada subida se guarda en S3, se escribe en PostgreSQL y se anuncia en SQS. Su p95 en un día normal ronda los 171 ms, pero cuesta unos 7364 $ al mes, muy por encima del presupuesto de 4000 $. Solo S3 son unos 4464 $ y SQS unos 1825 $, los dos cobrados por petición. Falla todas las pruebas por coste; cuando se sincroniza el portátil nuevo, la factura llega a unos 13 689 $.

Además, es lento. Cada subida ocupa un hueco de la Sync API mientras S3 responde. Las 6 instancias tienen 384 huecos, y en un día normal cerca del 40 %, unos 150, están ocupados. Cuando S3 añade 150 ms, 1700 subidas por segundo ocupan unos 255 huecos más (1700 × 0,15 s), más de los que quedan libres. Las peticiones hacen cola en la API y el p95 llega a unos 1,6 s, por encima del límite de 1 s.

La deduplicación por contenido, calcular el hash del bloque y saltárselo si ya lo tienes, ahorra dinero y tiempo a la vez.

## Lo que el simulador deja fuera

Un servicio real nombra cada bloque con un hash de su contenido, así que los mismos bytes reciben siempre el mismo nombre, los envíe quien los envíe. El simulador usa la clave de la petición como identidad del bloque. Tampoco se simulan los conflictos, cuando dos dispositivos editan el mismo archivo a la vez, ni la codificación por diferencias, en la que un dispositivo envía solo los bytes que han cambiado en lugar de bloques enteros.

## Para probar

- Baja el TTL del índice de bloques a 60 s y ejecuta «Un día normal». Observa cómo suben los costes de S3 y SQS a medida que aciertan menos subidas.
- En el primer intento, duplica las instancias de la Sync API y ejecuta «El almacenamiento de objetos se ralentiza». Comprueba si los huecos extra bastan, y qué dice la factura.
