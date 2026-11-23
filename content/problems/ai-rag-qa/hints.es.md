## 1

El modelo de chat es la parte cara, pero no es lo único que espera cada pregunta. ¿Cuántas búsquedas por segundo puede ejecutar un shard de tu Base de datos vectorial, y cuánto se acerca a eso un día normal? ¿Qué le pasa a una pregunta cuando el único nodo vectorial al que puede preguntar desaparece?

## 2

Un shard ejecuta 16 búsquedas a la vez, y una búsqueda sobre mil millones de vectores tarda unos 70 ms, así que un shard llega como mucho a algo más de 200 búsquedas por segundo. Más shards añaden huecos de búsqueda. Un segundo nodo conectado con Replicar también añade huecos, y sigue respondiendo cuando se pierde el primero. Pero cuando se pierde un nodo, el superviviente carga solo con todas las búsquedas, así que necesita huecos suficientes para eso.

## 3

Pon un AWS ALB delante de 4 Go App Servers con una concurrencia de 512 por instancia (cada uno conserva su hueco durante toda la respuesta). Calcula los embeddings en NVIDIA Triton, 2 instancias, tipo de modelo «Embeddings», velocidad del prompt de 50 000 tokens/s. Busca en una Base de datos vectorial Qdrant con 2 shards y 1000 millones de vectores, conectada con Replicar a un segundo Qdrant con los mismos ajustes, y consulta los dos desde los App Servers. Responde con vLLM en GPU, 12 instancias con 128 huecos de lote por GPU. Pon el tiempo límite del cliente web en 15 000 ms.
