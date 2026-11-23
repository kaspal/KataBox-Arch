## 1

Cuenta lo que llega al modelo de chat. A 200 preguntas por segundo, ¿cuántas peticiones por minuto son, y qué tan cerca están del límite de 16 000? Después piensa en una prueba con el doble o el triple de tráfico que dure más de un minuto. ¿Qué preguntas tiene que responder de verdad el modelo?

## 2

Las preguntas populares llegan palabra por palabra, y una respuesta se puede reutilizar durante 10 minutos. Una caché con la pregunta exacta como clave, delante de todo el pipeline, responde cada repetición dentro de esa ventana sin búsqueda, sin embedding y sin llamar al modelo. Cuanto más calientes son las preguntas, más responde. Comparar por significado solo atrapa la cuarta parte de preguntas que son casi repetidas, por caliente que esté el tema.

## 3

Usa un AWS ALB delante de 8 Servidores de aplicaciones Go con una Concurrencia por instancia de 512 (cada uno guarda su plaza durante toda la respuesta). Lee primero una Caché Redis: Modo "Claves exactas", sus 2 GB por defecto y un TTL de 600 s. Si falla, consulta un Índice de búsqueda Elasticsearch con sus valores por defecto, calcula el embedding en NVIDIA Triton con Tipo de modelo "Embeddings" y una Velocidad del prompt de 50 000 tokens/s, busca en una Base de datos vectorial Qdrant con 2 shards y 100 millones de vectores, y responde con la API de Claude con los límites de la cuenta (16 000 peticiones y 30 000 000 tokens por minuto).
