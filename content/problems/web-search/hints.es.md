## 1

Dos cosas pueden tumbar un buscador: una máquina que se para y una multitud que hace la misma pregunta a la vez. Si tu índice de búsqueda vive en una sola tarjeta, ¿qué responde las búsquedas cuando esa tarjeta ya no está? ¿Y qué parte del índice responde la consulta más popular?

## 2

Un índice de búsqueda envía cada consulta a la partición que la contiene, así que una consulta popular cae siempre en la misma partición. Más particiones reparten muchas consultas, pero nunca dividen una. Pon una caché delante del índice para la cabeza de la distribución: aquí no pasa nada si los resultados populares tienen unos minutos. Después guarda una segunda copia del índice, para que perder un nodo signifique tener menos máquinas, no quedarse sin resultados.

## 3

Un AWS ALB delante de 8 Go App Servers. Las búsquedas pasan en cache-aside por una Redis Cache con 8 GB de memoria y un TTL de 300 s. Detrás, una tarjeta Elasticsearch con 6 shards, conectada con Replicar a una segunda tarjeta Elasticsearch con 6 shards: las búsquedas se reparten entre las dos y las escrituras del índice van al primario.
