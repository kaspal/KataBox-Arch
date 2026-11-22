## 1

La mayoría de las subidas son bloques que ya tienes. ¿Qué te cuesta, en tiempo y en dinero, guardar otra vez uno de esos bloques, escribir otra vez su registro y volver a avisar de él a todos los dispositivos?

## 2

Dale a cada bloque una identidad y mantén un índice de los bloques que tienes. Una consulta rápida antes de la subida te permite saltarte el almacenamiento de objetos, la escritura en la base de datos y el aviso de cambio cuando el bloque se repite. S3 y SQS cobran por petición, así que cada bloque que te saltas es dinero ahorrado, y una subida que nunca toca el almacenamiento de objetos no nota que se ralentice.

## 3

Pon junto a la Sync API una Cache Redis como índice de bloques, con el patrón cache-aside y un TTL largo, por ejemplo un día (86 400 s): 1 GB sobra para una entrada por bloque. Deja S3 para los bloques, PostgreSQL para los metadatos de los archivos con una réplica que reparta los listados de carpetas, y una Queue SQS con unos pocos Notifiers Celery detrás.
