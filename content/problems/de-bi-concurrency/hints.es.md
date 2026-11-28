## 1

Cuenta los GB que el almacén tiene que escanear cada segundo. 12 consultas por segundo de 0,1 GB cada una, menos las pocas que responde la Caché de resultados, son unos 1,1 GB/s, y un clúster escanea 1 GB/s, repartido entre las consultas que corren en él. ¿Qué pasa cuando llegan cuatro veces más? ¿Y qué les hace a los analistas que esperan en la cola una consulta que escanea 60 GB?

## 2

Los clústeres multiplican la velocidad de escaneo: cada uno añade 1 GB/s y 8 huecos. Dimensiónalos para la avalancha de las 9, no para una hora normal. Una consulta de exploración escanea 60 GB, así que en un almacén compartido incluso una sola se queda con toda la velocidad de escaneo durante segundos, y una racha de ellas deja a los analistas esperando en la cola. Dale a los notebooks un almacén propio, con clústeres suficientes para seguir el ritmo de la exploración: se suspende solo, así que no cuesta casi nada mientras nadie explora.

## 3

Pon el Tiempo límite del Cliente en 120 000 ms. Conecta el Servidor de aplicaciones (2 instancias Go) a un Almacén de datos Snowflake con 5 instancias, y el Microservicio (2 instancias) a un segundo Almacén de datos Snowflake con 32 instancias. Deja la Concurrencia por instancia en 8 y la Suspensión automática tras en 60 s en los dos.
