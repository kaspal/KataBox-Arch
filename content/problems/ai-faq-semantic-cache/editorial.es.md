# Solución comentada

## Adónde va la carga

Una pregunta usa unos 500 + 120 = 620 tokens. La cuenta permite 400 000 tokens por minuto, unos 6700 por segundo, así que como mucho unas 10,75 preguntas por segundo pueden llegar al modelo. El pico es de 12 por segundo: enviar todas las preguntas al modelo ya pasa el límite en un día normal, y cada pregunta enviada cuesta alrededor de un tercio de céntimo (500 tokens de entrada a 3 $ por millón y 120 de salida a 15 $ por millón).

Así que la pregunta es cuántas tiene que responder de verdad el modelo. El tráfico lo dice: el 60 % de las preguntas de cada día significa lo mismo que una anterior, y el 95 % de las del tema viral también. Pero el mismo significado rara vez es el mismo texto. Una caché de claves exactas solo acierta con las redacciones comunes e idénticas (aquí, alrededor de un tercio de las preguntas); una caché semántica compara significados y puede responder cualquier pregunta casi repetida que ya haya visto.

Las pruebas ocultas aprietan las dos mitades. "Una pregunta se hace viral: el triple de tráfico" multiplica el tema viral por 41, así que pasa a ser unos dos tercios de 36 preguntas por segundo, escritas de 50 000 formas distintas. "El pico de la tarde, y un nodo de caché se reinicia" lleva 1,5 veces el tráfico durante 180 s y mata la caché a los 20 s, durante 5 s.

## El diseño de referencia

El Cliente web envía las preguntas a una tarjeta de Servidor de aplicaciones Go (2 instancias). El Servidor de aplicaciones lee una Caché Redis con Modo "Semántica (casi repetidas)" y 2 GB de memoria, conectada con Replicar a una segunda Caché Redis con los mismos ajustes. Si falla la caché, llama a una tarjeta de Inferencia de modelos API de Claude con sus valores por defecto y guarda la respuesta en la caché. 100 000 redacciones a 8 KB cada una (la respuesta y su embedding) son unos 800 MB, así que 2 GB las guardan todas.

Cuesta unos 41 544 $ al mes: unos 41 332 $ de tokens del modelo, 140 $ de los Servidores de aplicaciones y 36 $ por cada Redis. Casi todo son tokens. En un día normal llegan al modelo unas 4,6 preguntas por segundo, unos 171 000 tokens por minuto, el 43 % del límite. El p95 es de unos 3144 ms, marcado por las preguntas que van al modelo; una búsqueda semántica añade unos 15 ms (el embedding y la búsqueda), mucho menos que cualquier llamada al modelo. El 0,14 % de errores son las respuestas más largas, que pasan del tiempo límite de 5 s del cliente.

En la prueba viral, las casi repetidas ayudan justo donde hace falta: llegan al modelo unas 5,8 preguntas por segundo, unos 215 000 tokens por minuto, aunque llegan el triple de preguntas. El p95 baja a unos 2579 ms, porque más respuestas salen de la caché, y los errores se quedan en el 0,16 %.

En la prueba del reinicio, el nodo caído hace fallar las peticiones que le llegan durante los 2 segundos más o menos que tardan los chequeos de salud en notarlo; después responde la réplica. Cuando el nodo vuelve, se sincroniza con la copia de la réplica en lugar de arrancar vacío. Los errores son de un 0,96 %, por debajo del 3 % de la prueba.

El reto de coste no encontró un diseño más barato que pase (no hay reto, tras 60 evaluaciones): los tokens del modelo son más del 99 % de la factura, y la parte de preguntas que puede responder una caché la fija el tráfico, no el diseño.

## Por qué falla el primer intento

Un primer intento habitual es el mismo Servidor de aplicaciones y el mismo modelo con una Caché Redis de 2 GB en su Modo por defecto, "Claves exactas", con el TTL por defecto de 300 s. Cuesta unos 69 319 $, porque unas dos terceras partes de las preguntas siguen llegando al modelo, y pasa "Un día normal" con un p95 de unos 3271 ms y unos 283 000 tokens por minuto, el 71 % del límite.

Cuando el tema se hace viral, la caché exacta apenas ayuda: 50 000 redacciones repartidas por igual casi nunca se repiten palabra por palabra. Llegan al modelo unas 32 preguntas por segundo, cerca de 1,2 millones de tokens por minuto, el triple del límite. El cubo de tokens guarda el permiso de un minuto, así que el primer medio minuto todavía pasa; a partir de unos 26 s, alrededor de la mitad de las llamadas recibe un 429, y la prueba termina con un 43 % de errores más o menos.

La prueba del reinicio no lo atrapa. La caché vuelve vacía, pero cada respuesta del modelo se guarda de nuevo en ella, y las redacciones comunes se vuelven a preguntar en segundos, así que pronto vuelve a responder su tercio de las preguntas. Fallan las peticiones que llegan al nodo caído en los 2 segundos antes de que los chequeos de salud lo noten, y la prueba termina con alrededor de un 1,6 % de errores, por debajo de su 3 %. Pasa solo porque la prueba es corta: con 1,5 veces el tráfico, la caché exacta sigue enviando al modelo algo más de su límite de tokens por minuto, así que el cubo se vacía poco a poco. En una ejecución de 25 minutos los 429 empiezan a los 5 minutos más o menos, y termina con alrededor de un 4,4 % de errores.

Una sola caché semántica, sin la réplica, falla la prueba del reinicio (alrededor de un 21 % de errores): tiene que volver a ver las preguntas antes de poder emparejarlas, y volver a ver 100 000 redacciones a 18 por segundo lleva horas.

La idea es esta: delante de un modelo alojado, la caché decide cuánto gastas de tu límite de tokens y de tu factura. Compara las preguntas por su significado, porque así es como la gente las repite, y mantén viva la copia de esa caché, porque sin ella todo el tráfico cae a la vez sobre el modelo.

## Lo que el simulador deja fuera

El simulador modela las tasas de acierto, la latencia de la búsqueda, los límites de tokens y su coste, no la calidad de las respuestas. Una caché semántica real puede devolver una respuesta equivocada a una pregunta que solo se parece, y el umbral de similitud que equilibra aciertos y esos errores no es un ajuste aquí: la tasa de acierto la declara el tráfico y ningún diseño puede subirla. El paso del embedding son 15 ms fijos y no se factura. Las respuestas de la caché nunca se quedan anticuadas cuando cambia un artículo de ayuda. La latencia se cuenta hasta el último token; el streaming no se simula. La ventana de entrega de la API por lotes no se simula como tiempo; en su lugar, una llamada por lotes para una pregunta que alguien está esperando falla. La tarjeta acepta cualquier límite de uso que escribas, por eso el enunciado los fija. Los precios son los precios didácticos del registro para un modelo alojado de clase Sonnet. La persistencia de Redis no se simula, así que un Redis solo siempre vuelve vacío. Los dos tipos de caché se rellenan de forma distinta aquí. Una caché de claves exactas se llena clave a clave: empieza caliente (está en ella cada redacción que se pregunta al menos una vez por TTL), y cada respuesta del modelo se guarda de nuevo en ella, así que tras un reinicio las redacciones populares vuelven en segundos. El rellenado de una caché semántica es una curva más simple: acierta una casi repetida en proporción a cuántas de las redacciones distintas ha visto desde que se vació, sin favorecer las populares, así que reaprende más despacio de lo que lo haría una real.

## Para probar

- Cambia las cachés de la referencia a "Claves exactas" y ejecuta la prueba viral. Compara los tokens por minuto con el límite de 400 000.
- Quita la réplica y ejecuta la prueba del reinicio. Fíjate en cuándo empiezan los 429 y en cuánto tardaría la caché en volver a ver 100 000 redacciones.
- Envía todas las preguntas directamente al modelo, sin caché, y ejecuta "Un día normal". Razona por qué falla a partir de 620 tokens × 12 preguntas por segundo.
