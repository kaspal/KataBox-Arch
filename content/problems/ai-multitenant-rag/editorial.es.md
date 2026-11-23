# Solución comentada

## Adónde va la carga

Cada pregunta, la envíe el inquilino que la envíe, hace las mismas tres llamadas: el embedding, la búsqueda vectorial y el modelo de chat. Las tarjetas que hay detrás las comparten todos los inquilinos, así que su capacidad también se comparte. Nada en una cola sabe de quién es la pregunta.

La búsqueda vectorial cuesta el tiempo de servicio de la tarjeta, 2 ms, por 1 + log2 de los vectores por shard en millones, por √(vecinos ÷ 10). Con 500 millones de vectores en un shard y 50 vecinos, eso es 2 × (1 + log2 500) × √5 ≈ 2 × 10,0 × 2,24 ≈ 45 ms. Un shard ejecuta 16 búsquedas a la vez, así que termina como mucho unas 16 ÷ 0,045 ≈ 360 búsquedas por segundo. Un día normal envía 100.

El modelo de chat es el recurso compartido más grande. En vLLM una respuesta espera 200 ms, lee el prompt a 5000 tokens por segundo (unos 0,3 s para 1500 tokens) y después escribe 200 tokens a 80 tokens por segundo (unos 2,5 s) cuando la GPU está libre. A medida que se llenan los huecos de lote de una GPU, todas sus respuestas se ralentizan, hasta la mitad de velocidad cuando está llena. Así que un inquilino que envía más preguntas no solo ocupa huecos: hace más lentas las respuestas de todos los demás.

Las dos pruebas ocultas tiran en sentidos opuestos. En «El inquilino grande trae a todos sus equipos», el inquilino grande envía 4 veces su tráfico durante toda la prueba: 80 preguntas por segundo en lugar de 20, unas 160 en total. Son clientes que pagan, así que no se puede rechazar ninguna. En «El script de un inquilino inunda el servicio», el inquilino mediano envía 50 veces su tráfico durante toda la prueba: 500 preguntas por segundo, unas 590 en total.

Un límite por inquilino tiene que quedar entre las dos: por encima de 80 por segundo, para que pase el inquilino grande, y el contrato dice 100. Pero un límite de 100 aún deja al inquilino que inunda enviar 100 por segundo. Las tarjetas compartidas tienen que cargar con el tráfico normal de todos los demás más un inquilino en su límite: 20 + 70 + 100 = 190 preguntas por segundo. Esa cifra, y no las 100 de un día normal, marca el tamaño de las GPU.

## El diseño de referencia

El cliente web (tiempo límite de 15 000 ms) envía cada pregunta a través de un Limitador de tasa, Token bucket en Redis, con un Límite de 100 y una Ráfaga de 100, y después de un AWS ALB. El limitador guarda un bucket por cada clave de API, así que cada uno de los 402 inquilinos tiene sus propias 100 por segundo. Detrás del ALB, 4 Servidores de aplicaciones Go con una concurrencia de 512 por instancia conservan su hueco durante toda la respuesta. El embedding se calcula en NVIDIA Triton, 1 instancia con el tipo de modelo «Embeddings» y una velocidad del prompt de 50 000 tokens/s. La búsqueda se hace en una Base de datos vectorial Qdrant con 1 shard y 500 millones de vectores. La respuesta sale de vLLM en GPU, 10 instancias con 128 huecos de lote por GPU.

Cuesta unos 8669 $ al mes: 7340 $ las 10 GPU de chat, 734 $ la GPU de embeddings, 280 $ los Servidores de aplicaciones, 260 $ Qdrant, 30 $ el limitador y 25 $ el ALB.

En un día normal, las GPU de chat están ocupadas en torno al 28 %, Qdrant en torno al 33 % y los Servidores de aplicaciones en torno al 18 %. La respuesta entera tarda unos 6541 ms en el p95, el tiempo hasta el primer token queda por debajo de 0,9 s en el p95, y falla el 0,017 % de las preguntas.

Cuando el inquilino grande trae a sus equipos, sus 80 preguntas por segundo quedan por debajo de su límite de 100, así que no se rechaza nada. Las GPU están ocupadas en torno al 50 % y Qdrant en torno al 54 %. La respuesta entera tarda unos 7366 ms en el p95, y no falla ninguna pregunta.

Cuando el script inunda, el limitador deja pasar 100 de sus 500 preguntas por segundo y rechaza las otras 400 con un 429, que cuenta como un éxito para ese inquilino. Las tarjetas compartidas ven unas 190 preguntas por segundo: las GPU están ocupadas en torno al 66 %, Qdrant en torno al 64 % y los Servidores de aplicaciones en torno al 43 %. La respuesta entera tarda unos 8295 ms en el p95, y falla el 0,017 % de las preguntas.

Cada parte del diseño hace falta, y las variantes lo muestran:

- Con el mismo diseño y solo 8 GPU de chat, supera la llegada de los equipos (p95 de unos 8461 ms) pero no la avalancha: las 100 por segundo que el límite aún deja pasar llenan las GPU, todas las respuestas se ralentizan y el p95 llega a unos 12 573 ms.
- Con un Límite de 60, la avalancha pasa, pero la llegada de los equipos falla: se rechazan unas 20 de las 80 preguntas por segundo del inquilino grande, en torno al 11,4 % de todas las preguntas.
- Con un Límite de 150, la llegada de los equipos pasa, pero la avalancha deja entrar 150 por segundo y el p95 llega a unos 12 825 ms.
- Sin ningún limitador, el dinero no compra un aprobado dentro del presupuesto: 24 GPU de chat y 2 shards (19 175 $) aún fallan en torno al 41,0 % de las preguntas en la avalancha, y hacen falta 30 GPU, 3 shards y 8 instancias de Servidor de aplicaciones para superarla, por unos 24 119 $.

Para el reto de coste, la búsqueda de diseños de prueba no encontró nada por debajo del 90 % del coste de la referencia (7795 $): las GPU de chat son casi toda la factura, y la avalancha las necesita. Este problema no tiene reto de coste.

## Por qué falla el primer intento

Un primer intento habitual no tiene Limitador de tasa y tiene 8 GPU de chat, dimensionadas para la llegada de los equipos con algo de margen. Cuesta unos 7171 $. Supera «Un día normal» (p95 de unos 6805 ms, 0,017 % de errores) y «El inquilino grande trae a todos sus equipos» (p95 de unos 8461 ms, 0,094 % de errores, con las GPU ocupadas en torno al 71 %).

Cuando el script inunda, las 590 preguntas por segundo llegan a las tarjetas compartidas. En los primeros segundos Qdrant recibe 590 búsquedas por segundo frente a las 360 o así que puede terminar su shard: su cola se llena, y a lo largo de la prueba se descartan unas 21 000 búsquedas. Las preguntas que pasan llenan las GPU, que trabajan en torno al 97 %, así que todas las respuestas se ralentizan hasta tardar más que los 15 segundos de los clientes. Los Servidores de aplicaciones, que conservan un hueco por cada una de esas respuestas lentas, están ocupados en torno al 92 %. Falla en torno al 84,2 % de todas las preguntas.

Los demás inquilinos no hicieron nada mal, y también fallan. Sus preguntas son iguales que las del script (el mismo camino, los mismos tamaños de prompt y de respuesta), y esperan en las mismas colas de las mismas tarjetas, así que fallan más o menos igual de a menudo. Ejecuta la misma prueba quitando el tráfico del inquilino que inunda y este diseño no tiene ningún error.

La idea es esta: en un servicio compartido, el tráfico de un inquilino es la latencia de todos. Cuando no puedes darle a cada inquilino sus propias máquinas, dale a cada uno una cuota, fíjala a partir de lo que prometes a tu inquilino más grande, y dimensiona la capacidad compartida para el tráfico normal más un inquilino en esa cuota.

## Lo que el simulador deja fuera

No juzga las respuestas: no se simula si los 50 pasajes encontrados son los correctos ni si la respuesta es correcta. Tampoco separa los datos de los inquilinos: en un servicio real cada búsqueda solo debe ver los documentos de su propio inquilino (un filtro o un índice por inquilino), y una fuga entre inquilinos es un fallo de seguridad que el simulador no ve. Cada tipo de petición se reparte entre todas las tarjetas que encajan, así que los movimientos reales de aislamiento (un índice o un shard aparte para el inquilino grande, grupos de GPU aparte por nivel de inquilino, una cola justa o con prioridad por inquilino en el servidor de modelos) no se pueden construir aquí; una cuota por clave de API es el aislamiento que ofrece el simulador. Un solo Límite se aplica a todas las claves, mientras que los planes reales suelen tener niveles. El limitador cuenta preguntas, no tokens. El tiempo de búsqueda es una decisión de modelado (una fórmula sobre los vectores por shard, los vecinos y los shards), no una medida de Qdrant; las dimensiones de los vectores, la memoria y las actualizaciones del índice no se simulan. El simulador saca un único tamaño de prompt por pregunta y lo cobra en las dos llamadas a modelos, así que la velocidad del prompt de la tarjeta de embeddings está alta para representar una pregunta corta. El tiempo hasta el primer token se mide en cada llamada a un modelo, así que su p95 mezcla las llamadas de embeddings con las de chat. El envío de tokens por la red, la memoria de la GPU y la caché KV no se simulan. El precio de la GPU es el de una máquina de la clase g5.xlarge, y el precio de la Base de datos vectorial es una cifra didáctica que aún necesita una fuente.

## Cosas que probar

- Quita el Limitador de tasa y ejecuta «El script de un inquilino inunda el servicio». Fíjate en los descartes de Qdrant en los primeros segundos y después en la ocupación de las GPU.
- Pon el Límite a 60 y luego a 150, y ejecuta las dos pruebas ocultas cada vez. ¿Cuál falla, y por qué?
- Deja el limitador y baja las GPU de chat a 8. La llegada de los equipos sigue pasando; averigua por qué no pasa la avalancha, a partir de las 190 preguntas por segundo y de cómo se ralentizan las GPU al llenarse.
