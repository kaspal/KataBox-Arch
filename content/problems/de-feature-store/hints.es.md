## 1

Tres tipos de trabajo comparten un producto, y quieren cosas muy distintas: 800 lecturas por segundo que deben responder en milisegundos, unas 200 actualizaciones por segundo que deben llegar casi al momento, y 2 lecturas grandes de entrenamiento por segundo que pueden tardar un segundo. ¿Qué almacén está hecho para cada una? ¿Y cuántas lecturas por segundo aguanta un almacén cuando cada consulta cuesta 300 ms y una instancia ejecuta 8 a la vez?

## 2

Una caché delante del almacén de datos solo ayuda con los usuarios que vuelven dentro de su TTL. Cada fallo es una consulta de 300 ms al almacén de datos, y más tráfico significa más fallos a la vez. Pon las características de todos los usuarios en un almacén en línea que responda cada lectura en pocos milisegundos, y deja que el procesador de streams escriba las actualizaciones directamente en él; guarda el almacén de datos para las lecturas de entrenamiento. Después da margen al camino del stream: si cada actualización tarda 100 ms más, ¿cuántas tienen que estar en curso para seguir el ritmo de 200 por segundo?

## 3

Pon un AWS ALB delante de 2 App Servers Go y de un microservicio Genérico para las lecturas de entrenamiento. Los App Servers leen de un almacén NoSQL ScyllaDB con 1 partición y publican los eventos en un Stream Apache Kafka con 32 particiones. Apache Flink, 4 instancias con una concurrencia de 8 (32 huecos), consume el Stream y escribe en el mismo ScyllaDB. El microservicio consulta un almacén de datos Snowflake con 3 instancias, escaneo suficiente para el triple de lecturas de entrenamiento.
