Una red de distribución de contenidos (CDN) es un conjunto de cachés repartidas por el mundo, en el **borde** de la red, cerca de los usuarios. La petición de un fichero va al borde más cercano. Si el borde tiene una copia fresca (un **acierto**), responde en pocos milisegundos. Si no (un **fallo**), pide el fichero a tu **origen**, como un almacenamiento de objetos o un servidor de aplicaciones, y guarda una copia para el siguiente usuario.

Funciona mejor con contenido que es igual para todos: imágenes, scripts, hojas de estilo, fragmentos de vídeo. El origen solo ve entonces los fallos, así que una CDN puede convertir una avalancha de lecturas en un goteo.

En el simulador una CDN tiene capacidad ilimitada y solo cachea los tipos de petición que el problema marca como cacheables; todo lo demás pasa al origen. Las claves populares aciertan mucho más que las raras, y cada copia caduca pasado el TTL.
