Los comandos, las peticiones que cambian cosas, van a un modelo de escritura que comprueba las reglas y guarda el resultado en una base de datos normalizada. Cada cambio se publica además como un evento.

Los proyectores consumen esos eventos y mantienen al día uno o más modelos de lectura: un documento por pantalla, un índice de búsqueda, una lista ordenada. Las consultas van solo a los modelos de lectura, preparados para que cada respuesta sea una búsqueda barata.

El lado de lectura siempre va algo por detrás, así que un usuario puede no ver su propio cambio enseguida. Donde importe, muestra el resultado desde la respuesta del comando, o lee del lado de escritura para ese usuario. Para publicar el evento de forma fiable junto con la escritura, combina CQRS con un outbox transaccional.
