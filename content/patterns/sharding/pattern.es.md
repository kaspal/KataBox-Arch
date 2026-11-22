Cada registro tiene una clave de shard, como un id de usuario. Una función de esa clave, normalmente un hash, elige qué shard es dueño del registro, y cada lectura o escritura de esa clave va allí. Cada shard es una base de datos completa con sus propios servidores, así que añadir shards añade capacidad.

La elección de la clave lo decide todo. Una buena clave reparte las peticiones por igual y mantiene en un solo shard los datos que necesita una petición. Una mala manda el tráfico de un famoso, o la fecha de hoy, a un único shard que se satura mientras el resto está ocioso.

El trabajo que abarca muchas claves se complica: una consulta sobre todos los usuarios tiene que preguntar a cada shard y combinar las respuestas. Muchos almacenes NoSQL hacen el sharding por ti con la clave de partición; una base de datos SQL suele necesitar que la app o una capa de enrutado sepa dónde vive cada clave.
