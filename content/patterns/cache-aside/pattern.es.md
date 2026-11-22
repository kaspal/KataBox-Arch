La app pregunta a la caché por una clave antes de ir a ningún otro sitio. Si la clave está (un acierto), la respuesta vuelve directamente y la base de datos ni se entera. Si no está (un fallo), la app lee la base de datos, guarda el resultado en la caché con un tiempo de vida y después responde.

Las escrituras van a la base de datos. La copia en caché se queda como está, así que los lectores pueden ver el valor antiguo hasta que caduque, salvo que la app también borre la clave al escribir. Un TTL corto limita lo obsoleta que puede estar una lectura; uno largo da más aciertos.

La caché solo se llena con claves que alguien ha pedido, así que su memoria va a los datos que importan. El precio es la ruta del fallo: tras un reinicio o un vaciado, cada lectura falla y la base de datos carga con todo hasta que vuelven las claves calientes.
