Publicar/suscribir, o **pub/sub**, entrega un mensaje a muchos receptores. Quien publica envía un mensaje a un **topic**, y el topic manda una copia a cada **suscriptor**. Quien publica no sabe quién escucha, así que pueden unirse suscriptores nuevos sin ningún cambio por su parte.

Es la forma habitual de unir servidores que tienen, cada uno, parte de las conexiones en vivo de los usuarios. Un mensaje de chat llega a un servidor, que lo publica; cada servidor suscrito a la conversación lo recibe y lo pasa a sus propios usuarios.

Pub/sub se diferencia de una cola en dos cosas. Cada suscriptor recibe todos los mensajes, así que el trabajo crece con el número de suscriptores. Y muchos topics guardan poco o nada, así que un suscriptor lento o desconectado o bien frena a todos o bien pierde mensajes. Cuando un suscriptor necesita un búfer, pon una cola entre él y el topic.
