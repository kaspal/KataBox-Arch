# Solución comentada

## Adónde va la carga

Cada pregunta hace tres llamadas seguidas: el embedding, la búsqueda vectorial y el modelo de chat. El modelo de chat se lleva la mayor parte del tiempo y del dinero, pero escala de la forma habitual: más GPU, más huecos de lote. El paso que es fácil olvidar es la búsqueda vectorial, porque una sola búsqueda parece barata.

Es barata en tiempo, no en huecos. En el simulador, una búsqueda cuesta el tiempo de servicio de la tarjeta, 2 ms, por 1 + log2 de los vectores por shard en millones, por √(vecinos ÷ 10). Con 1000 millones de vectores en un shard y 100 vecinos, eso da 2 × (1 + log2 1000) × √10 ≈ 2 × 11,0 × 3,16 ≈ 69 ms. Un shard ejecuta 16 búsquedas a la vez, así que termina como mucho unas 16 ÷ 0,069 ≈ 230 búsquedas por segundo, y menos si se cuenta la cola lenta. Un día normal envía 150. Un solo shard ya está ocupado en torno a tres cuartas partes del tiempo.

Partir el índice en shards apenas acorta una búsqueda: con 2 shards cada uno guarda 500 millones de vectores, pero responden todos y el más lento marca el tiempo, así que una búsqueda sigue tardando unos 70 ms. Lo que añaden los shards son huecos: 2 shards ejecutan 32 búsquedas a la vez. Un segundo nodo conectado con Replicar también añade huecos, y es lo único que mantiene las búsquedas respondidas cuando se pierde un nodo.

Las pruebas ocultas comprueban las dos cosas. «Las preguntas se duplican durante 30 s» envía 300 preguntas por segundo del segundo 15 al 45. «Se pierde un nodo de búsqueda vectorial el día del lanzamiento» ejecuta toda la prueba a 1,5 veces el pico (225 preguntas por segundo) y mata la primera tarjeta de Base de datos vectorial en el segundo 20, durante 30 segundos. Mientras está caída, lo que sobreviva carga con las 225 búsquedas por segundo.

## El diseño de referencia

Un AWS ALB reparte las preguntas entre 4 Go App Servers con una concurrencia de 512 por instancia. Conservan su hueco durante toda la respuesta, unos 5 s en el p95, así que necesitan muchos huecos, no mucha CPU. El embedding se calcula en NVIDIA Triton, 2 instancias con el tipo de modelo «Embeddings» y una velocidad del prompt de 50 000 tokens/s. La búsqueda se hace en una Base de datos vectorial Qdrant con 2 shards y 1000 millones de vectores, conectada con Replicar a un segundo Qdrant con los mismos ajustes, y los App Servers consultan los dos. La respuesta sale de vLLM en GPU, 12 instancias con 128 huecos de lote por GPU. El tiempo límite del cliente web es de 15 000 ms, para que una respuesta larga no se corte.

Cuesta unos 11 621 $ al mes: 8808 $ las 12 GPU del modelo de chat, 1468 $ las 2 GPU de embeddings, 520 $ cada Qdrant (2 shards cada uno), 280 $ los App Servers y 25 $ el ALB. El modelo de chat es tres cuartas partes de la factura; toda la búsqueda vectorial no llega a una décima parte.

En un día normal, cada Qdrant está ocupado en torno al 20 %, las GPU de chat en torno al 27 % y los App Servers en torno al 21 %. El tiempo hasta el primer token es de unos 0,7 s en el p95, la respuesta entera de unos 4957 ms en el p95, y no hay errores.

Cuando las preguntas se duplican, las dos tarjetas Qdrant suman 64 huecos de búsqueda y quedan lejos de llenarse. El tiempo hasta el primer token se mantiene en unos 0,7 s en el p95, la respuesta entera tarda unos 5924 ms en el p95 (las GPU reparten su velocidad entre más respuestas) y falla el 0,007 % de las preguntas.

Cuando se pierde un nodo el día del lanzamiento, falla en torno al 1,66 % de las preguntas, todas en los 2 segundos más o menos que tardan las comprobaciones de salud en notar el nodo caído. La prueba admite un 3 % de errores y un 97 % de disponibilidad. Después, todas las búsquedas van al Qdrant que sobrevive, cuyos 2 shards ejecutan 32 búsquedas a la vez: 225 búsquedas por segundo lo mantienen ocupado algo más de la mitad del tiempo. El tiempo hasta el primer token se mantiene en unos 0,7 s en el p95 y la respuesta entera en unos 5366 ms.

Hacen falta las dos mitades. Un Qdrant con 2 shards y sin réplica supera el pico, pero falla en torno al 49,3 % de las preguntas cuando se pierde. Dos tarjetas Qdrant con 1 shard cada una también superan el pico, pero cuando se pierde una, la superviviente tiene 16 huecos para 225 búsquedas por segundo: se desborda y falla en torno al 7,6 % de las preguntas.

Para el reto de coste, la búsqueda de diseños de prueba no encontró nada por debajo del 90 % del coste de la referencia (10 487 $): las GPU marcan la factura y tienen poco margen. Este problema no tiene reto de coste.

## Por qué falla el primer intento

Un primer intento habitual es el mismo diseño con un solo Qdrant de 1 shard y sin réplica. Cuesta unos 10 841 $ y supera «Un día normal»: su único shard está ocupado en torno al 78 %, y el p95 es de unos 4957 ms sin errores.

Cuando las preguntas se duplican, 300 búsquedas por segundo llegan a un shard que termina unas 230 como mucho. Su cola se llena, y las preguntas que la encuentran llena se rechazan: falla en torno al 23,0 % de las preguntas, casi todas durante los 30 segundos del pico. El modelo de chat, más grande y más caro, ni siquiera las ve.

Cuando se pierde el nodo el día del lanzamiento, no hay nada más a lo que preguntar. A 1,5 veces el pico, el único shard ya está por encima de su límite y falla algunas preguntas antes de la caída; después, durante 30 segundos, fallan todas. En total falla en torno al 57,1 % de las preguntas.

La idea es esta: en un pipeline de recuperación cada pregunta pasa por la búsqueda vectorial, así que hay que dimensionarla como cualquier otro servidor. Los shards y las réplicas añaden huecos de búsqueda, pero solo una réplica mantiene el índice respondiendo cuando se pierde un nodo, y la superviviente necesita huecos para cargar sola con todo.

## Lo que el simulador deja fuera

No juzga las respuestas: no se simula si los 100 pasajes encontrados son los correctos (el recall) ni si la respuesta es correcta, y nada de esto dice cuántos vecinos o qué modelo dan buenas respuestas. El tiempo de búsqueda es una decisión de modelado (una fórmula sobre los vectores por shard, los vecinos y los shards), no una medida de Qdrant; las dimensiones de los vectores, la memoria, el tiempo de construir el índice y sus actualizaciones no se simulan. El simulador saca un único tamaño de prompt por pregunta y lo cobra en las dos llamadas a modelos, así que la velocidad del prompt de la tarjeta de embeddings está alta para representar una pregunta corta. El tiempo hasta el primer token se mide en cada llamada a un modelo, así que su p95 mezcla las llamadas de embeddings (cortas) con las de chat. El envío de tokens por la red, la memoria de la GPU y la caché KV no se simulan. El precio de la GPU es el de una máquina de la clase g5.xlarge, y el precio de la Base de datos vectorial es una cifra didáctica que aún necesita una fuente.

## Cosas que probar

- Pon las tarjetas Qdrant en 1 shard cada una y ejecuta «Se pierde un nodo de búsqueda vectorial el día del lanzamiento». Fíjate en la cola de la tarjeta que sobrevive.
- Quita la réplica y dale 4 shards al único Qdrant. ¿Qué prueba oculta sigue fallando, y por qué no la arreglan más shards?
- Baja las GPU de chat a 8 y ejecuta «Las preguntas se duplican durante 30 s». Compara el tiempo hasta el primer token con el p95 de la respuesta entera.
