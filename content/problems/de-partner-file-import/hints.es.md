## 1

La subida en sí es fácil: el App Server guarda el fichero y encola un mensaje. Lo difícil pasa después, en los Workers. ¿Qué le pasa a un mensaje cuya escritura en PostgreSQL falla, y cuánto tiempo sigue intentándolo? ¿Y cuánto trabajo espera en la cola cuando los ficheros son 25 veces más grandes?

## 2

Un failover de la base de datos tarda unos 30 segundos, y los 3 reintentos por defecto de la cola se rinden tras 7. Sube los «Reintentos máximos» hasta que las esperas sumen más que un failover, y dale a la base de datos una réplica que pueda tomar el relevo. Conecta una Cola de mensajes fallidos para lo que aún falle, pero trata cada fichero que acabe ahí como un fichero sin cargar. Después dimensiona los Workers para los huecos que ocupan los ficheros gigantes, no para un día normal.

## 3

Conecta el servidor del socio a 2 App Servers Go, que escriben en Amazon S3, leen PostgreSQL y publican en una Cola Amazon SQS con «Reintentos máximos» en 8 (1 + 2 + 4 + 8 + 16 + 32 + 32 + 32 = 127 s de espera). Conecta la cola con Publicar a una Cola de mensajes fallidos. 16 Workers Celery (concurrencia 8) la consumen, leen S3 y escriben en PostgreSQL. Conecta PostgreSQL con Replicar a una segunda PostgreSQL, con el tiempo de failover de 30 s.
