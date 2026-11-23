## 1

Todos los inquilinos comparten la misma búsqueda vectorial y las mismas GPU. ¿Qué le pasa a la pregunta de un inquilino pequeño cuando un inquilino envía quinientas preguntas por segundo? ¿Y qué puedes limitar por inquilino, si no puedes darle a cada uno sus propias tarjetas?

## 2

Un Limitador de tasa guarda un bucket por clave de API, así que un solo Límite frena a cada inquilino por separado. No puede ser menor que el tráfico del inquilino grande cuando traiga a sus equipos (unas 80 por segundo), o rechazarás a un inquilino que paga. El contrato dice 100. Después dimensiona las tarjetas compartidas para lo peor que un límite aún deja pasar: el tráfico normal de todos los demás más un inquilino a 100 por segundo, unas 190 preguntas por segundo. Las GPU frenan todas las respuestas a medida que se llenan, así que déjales margen.

## 3

Pon un Limitador de tasa, Token bucket en Redis, con un Límite de 100 y una Ráfaga de 100, entre el cliente web y un AWS ALB. Detrás, 4 Servidores de aplicaciones Go con una concurrencia de 512 por instancia. Calcula el embedding en NVIDIA Triton, 1 instancia, tipo de modelo «Embeddings», velocidad del prompt 50 000 tokens/s. Busca en una Base de datos vectorial Qdrant con 1 shard y 500 millones de vectores. Responde con vLLM en GPU, 10 instancias con 128 huecos de lote por GPU. Pon el tiempo límite del cliente web a 15 000 ms.
