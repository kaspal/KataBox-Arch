## 1

Una cola no hace desaparecer el trabajo; solo le deja esperar. El plazo se cumple mientras lo que se acumula en la cola se despache antes de tener 30 segundos. Así que hazte dos preguntas: ¿cuántos borrados pueden tener en curso los Workers a la vez, y cuántos puede terminar por segundo el almacén más lento?

## 2

Un borrado ocupa un hueco de Worker durante las tres escrituras, alrededor de medio segundo en un día normal. Si las escrituras en la tabla del lago tardan 3 s más, cada borrado ocupa su hueco siete veces más tiempo, y necesitas siete veces los huecos para el mismo tráfico. Aparte, el Almacén de datos termina como mucho «Concurrencia por instancia» ÷ 0,3 s borrados por segundo y por instancia: cuenta cuántos trae una avalancha. No necesitas sitio para toda la avalancha, solo el suficiente para que el atasco se vacíe con holgura dentro del plazo.

## 3

Conecta el cliente web a Servidores de aplicaciones Go (2 instancias, el valor por defecto), que publican en una Cola Amazon SQS con «Reintentos máximos» a 8, conectada con Publicar a una Cola de mensajes fallidos. 8 Workers Celery (concurrencia 8, 64 huecos) la consumen y escriben en una Tabla lakehouse Apache Iceberg, un Almacén de datos Snowflake con 2 instancias y un Almacén NoSQL DynamoDB.
