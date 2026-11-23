## 1

Cuenta los huecos del almacén. 12 consultas por segundo que tardan unos 0,4 s cada una mantienen unas 5 consultas en marcha a la vez, y un clúster ejecuta 8. ¿Qué pasa cuando llegan cuatro veces más? ¿Y qué les hace a los analistas que esperan detrás una consulta que ocupa un hueco durante un minuto entero?

## 2

Los clústeres multiplican los huecos: cada uno añade 8. Dimensiónalos para la avalancha de las 9, no para una hora normal. Las consultas de exploración ocupan sus huecos unos 60 s, así que en un almacén compartido una racha de ellas llena todos los huecos y los analistas esperan en la cola. Dale a los notebooks un almacén propio: se suspende solo, así que no cuesta casi nada mientras nadie explora.

## 3

Pon el Tiempo límite del Cliente en 120 000 ms. Conecta el Servidor de aplicaciones (2 instancias Go) a un Almacén de datos Snowflake con 3 instancias, y el Microservicio (2 instancias) a un segundo Almacén de datos Snowflake con 5 instancias. Deja la Concurrencia por instancia en 8 y la Suspensión automática tras en 60 s en los dos.
