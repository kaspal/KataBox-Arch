## 1

Cada transferencia cae en una sola base de datos antes de que pase nada más. ¿Qué responde la app mientras esa base de datos no está, y hay algo que pueda ocupar su lugar?

## 2

Dale al libro mayor una copia que se pueda promover cuando caiga el primario. Después piensa en los saldos: salen del stream de cambios del libro mayor, así que la copia que sea primaria en cada momento tiene que poder publicar en ese stream.

## 3

Conecta un segundo PostgreSQL al libro mayor con un enlace Replicate, y dales a los dos un enlace de captura de cambios al mismo stream Kafka, para que la réplica promovida siga publicando. Sube el stream muy por encima de sus 6 particiones por defecto, hasta unas 32, y deja que unos pocos proyectores Celery con un tiempo de servicio corto escriban los saldos en DynamoDB.
