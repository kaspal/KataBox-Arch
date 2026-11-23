## 1

Cada cambio tiene que esperar a una escritura en el almacén de unos 300 ms. ¿Cuántas escrituras tienen que estar en curso a la vez para seguir el ritmo de 12 cambios por segundo? ¿Y con 36 por segundo, o cuando cada escritura tarda 1,8 s?

## 2

Tres tarjetas ponen cada una un tope a los cambios en curso: el Stream entrega un registro por partición cada vez, el procesador de streams retiene un hueco durante toda la escritura, y el almacén de datos ejecuta un número fijo de escrituras por instancia. Lo que pasa en un segundo es el menor de los tres dividido por el tiempo de una escritura. Dimensiona los tres para el almacén lento, no para el día medio, y deja que el Stream guarde lo que sobre hasta que se pueda escribir.

## 3

Conecta PostgreSQL con una conexión Publish (captura de cambios) a un Stream Apache Kafka con 24 particiones. Consúmelo con Apache Flink, 2 instancias con una concurrencia de 12 (24 huecos), escribiendo en Snowflake con 3 instancias (8 escrituras cada una, 24 en total). Mantén un AWS ALB y 2 App Servers Go delante de la base de datos.
