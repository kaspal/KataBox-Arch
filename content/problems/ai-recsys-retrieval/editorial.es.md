# Editorial

## Adónde va la carga

Una petición que falla en la caché hace dos llamadas seguidas: la búsqueda vectorial que recupera 50 candidatos, y el clasificador que los reordena. El clasificador es la tarjeta más cara, pero es rápido: en NVIDIA Triton una llamada tarda unos 5 ms en empezar, 10 ms en leer 50 tokens y un poco más en su respuesta de un token, y una GPU tiene 64 huecos de lote. El paso que se agota primero es la búsqueda vectorial.

En el simulador una búsqueda cuesta el tiempo de servicio de la tarjeta, 2 ms, por 1 + log2 de los vectores por shard en millones, por √(vecinos ÷ 10), por 1 + 0,15 ln(shards). Con 5 millones de vectores en un solo shard y 50 vecinos, eso es 2 × (1 + log2 5) × √5 ≈ 2 × 3,3 × 2,2 ≈ 15 ms. Un shard ejecuta 16 búsquedas a la vez, así que termina como mucho unas 16 ÷ 0,015 ≈ 1070 búsquedas por segundo, y menos si cuentas la cola lenta. Un día normal envía hasta 600, y una caché apenas cambia eso: la popularidad de los artículos es una cola larga, así que menos de una petición de cada diez encuentra su lista en la caché.

Hay dos cosas que pueden romper la búsqueda, y cada prueba oculta comprueba una:

- **«Una rebaja triplica el tráfico»** ejecuta toda la prueba a 1800 peticiones por segundo. Son más búsquedas de las que puede ejecutar un shard, haga lo que haga la caché. La solución son más huecos: shards. Con 2 shards cada uno guarda 2,5 millones de vectores, una búsqueda tarda unos 2 × 2,3 × 2,2 × 1,1 ≈ 11,5 ms, y 32 huecos terminan unas 2800 búsquedas por segundo.
- **«Un artículo se hace viral»** hace la popularidad mucho más empinada durante toda la prueba (un artículo recibe cerca del 61 % de todas las visitas, y los siguientes casi todo el resto) y multiplica el tráfico por 6 del segundo 15 al 45: 3600 peticiones por segundo, unas 2200 de ellas para el mismo artículo. Las búsquedas de un artículo van siempre al mismo shard, así que ese shard recibe 2200 búsquedas por segundo. Ningún shard puede terminar tantas: añadir shards reparte los demás artículos, no este. La solución es dejar de enviar esas peticiones a la búsqueda. Todos los compradores que abren el artículo viral reciben la misma lista, así que una caché les responde a partir del primero.

## El diseño de referencia

Un AWS ALB envía las peticiones a 4 App Servers en Go. Primero preguntan a una Caché Redis con 4 GB de memoria y un TTL de 300 s; en 4 GB caben unos 2 millones de listas de 2 KB. Si falla, buscan en una Base de datos vectorial Qdrant con 2 shards y 5 millones de vectores, y después reordenan en NVIDIA Triton, 1 instancia con el tipo de modelo «Clasificador».

Cuesta unos 1608 $ al mes: 734 $ por la GPU de Triton, 520 $ por la tarjeta Qdrant (2 shards a 260 $), 280 $ por los App Servers, 49 $ por la Caché Redis y 25 $ por el ALB.

En un día normal la tarjeta Qdrant está ocupada cerca del 23 % del tiempo, los App Servers cerca del 9 % y la GPU cerca del 14 %. El p95 es de unos 62 ms y no hay errores.

Cuando una rebaja triplica el tráfico, los shards llevan el triple de búsquedas que un día normal y siguen lejos de llenarse: el p95 se queda en unos 62 ms, sin errores.

Cuando un artículo se hace viral, la caché responde casi todas las peticiones de los artículos populares, así que la ráfaga apenas llega a la búsqueda. El p95 baja a unos 18 ms (la mayoría de las respuestas salen directamente de Redis), sin errores.

Cada mitad cubre una prueba, y cada una por separado falla la otra:

- Shards sin caché: un Qdrant con 4 shards y sin Redis (2079 $) pasa la rebaja, pero cuando el artículo se hace viral su shard se desborda y falla cerca del 10,2 % de las peticiones. Incluso 16 shards (5199 $, por encima del presupuesto) siguen fallando cerca del 4,4 %: el shard del artículo viral sigue siendo un solo shard.
- Una caché sin shards: un Qdrant con 1 shard más la Caché Redis (1348 $) pasa la prueba viral, pero la cola larga de la rebaja falla en la caché y fallan cerca del 41,5 % de las peticiones.

Para el reto de coste, la búsqueda de pruebas no encontró nada por debajo del 89 % del coste de la referencia (1431 $): la única GPU es casi la mitad de la factura y no se puede partir, y la búsqueda y la caché ya son pequeñas. Este problema no tiene reto de coste.

## Por qué falla el primer intento

Un primer intento habitual es un Qdrant con 1 shard, sin caché, y los mismos App Servers y GPU de Triton. Cuesta unos 1299 $ y pasa «Un día normal»: su único shard está ocupado cerca del 66 % del tiempo, y el p95 es de unos 70 ms sin errores.

Cuando una rebaja triplica el tráfico, 1800 búsquedas por segundo llegan a un shard que termina unas 1000 como mucho. Su cola se llena, y las peticiones que la encuentran llena se rechazan: falla cerca del 49,8 % de las peticiones. La GPU del clasificador, la tarjeta más cara, nunca las ve.

Cuando un artículo se hace viral, ese mismo shard solitario recibe toda la ráfaga, y falla cerca del 64,1 % de las peticiones.

La idea es esta: en un pipeline de recuperar y reordenar, la recuperación es un servidor como cualquier otro, y falla de dos formas distintas. Demasiado tráfico en total necesita más shards. Demasiado tráfico para una sola clave necesita la respuesta en caché, porque una clave cae siempre en el mismo shard.

## Lo que el simulador deja fuera

No juzga las recomendaciones: no se simula si los 50 vecinos son los correctos (recall), si el clasificador los ordena bien, ni lo frescas que están las listas en caché comparadas con una lista recién reordenada, y nada de esto dice cuántos candidatos o qué modelo dan buenas recomendaciones. Las listas personalizadas (una por comprador) no se modelan: todos los compradores reciben la misma lista para un artículo, y eso es lo que permite que la caché ayude. El tiempo de búsqueda es una decisión de modelado (una fórmula sobre los vectores por shard, los vecinos y los shards), no una medición de Qdrant; no se simulan las dimensiones de los vectores, la memoria, la construcción del índice ni sus actualizaciones, y el catálogo no crece durante una prueba. Las bases de datos vectoriales reales suelen buscar en todos los shards en cada consulta; aquí las búsquedas de cada artículo van a un solo shard, y eso es lo que convierte un artículo viral en un shard caliente. El clasificador es una llamada por petición: puntúa los 50 candidatos como un solo lote de unos 50 tokens. No se simula la memoria de la GPU. El precio de la GPU es una cifra de tipo g5.xlarge, y el precio de la Base de datos vectorial es una cifra didáctica que aún necesita fuente.

## Cosas que probar

- Quita la caché y dale 4 shards a la tarjeta Qdrant. Ejecuta «Un artículo se hace viral» y mira qué shard se llena.
- Mantén la caché y vuelve a poner 1 shard en la tarjeta Qdrant. Ejecuta «Una rebaja triplica el tráfico»: ¿cuánto ayuda la caché cuando la mayoría de los artículos se ven poco?
- Pon el TTL de la Caché en 10 s y ejecuta «Un artículo se hace viral». ¿Sigue el artículo viral fuera de la búsqueda?
