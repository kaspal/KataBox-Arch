# Motor de respuestas web

## El producto

Gestionas un motor de respuestas web. Alguien escribe una pregunta y, en lugar de diez enlaces, recibe una respuesta breve y escrita con sus fuentes. Detrás de cada respuesta hay un pipeline: una búsqueda por palabras clave en el índice de la web, un embedding de la pregunta, una búsqueda de los pasajes más cercanos por significado y un modelo de chat alojado que escribe la respuesta con lo que se encontró. El motor es popular, así que los límites por minuto del modelo de chat y su factura de tokens son las dos cosas que pueden pararlo. Las respuestas son las mismas para todos los que hacen la misma pregunta, y cuando estalla una noticia, mucha gente hace la misma pregunta a la vez.

## Lo que debes cumplir

| Objetivo | Meta |
| --- | --- |
| Tráfico pico | 200 peticiones/s |
| Latencia p95 | 10 000 ms o menos |
| Disponibilidad | 99 % o más |
| Errores | 1 % o menos |
| Presupuesto | 5 000 000 $ al mes |

## El tráfico

- **Hacer una pregunta al motor de respuestas** (100 %). La petición puede pasar por tarjetas de borde (una CDN, un balanceador de carga) y llega a tus Servidores de aplicaciones. El Servidor de aplicaciones puede leer primero una caché (cache-aside). Si falla la caché, busca la pregunta en el índice de la web (una tarjeta de Índice de búsqueda), calcula su embedding (una tarjeta de Inferencia de modelos con Tipo de modelo "Embeddings"), pide a una Base de datos vectorial los 20 pasajes más cercanos y envía la pregunta con lo encontrado al modelo de chat (una tarjeta de Inferencia de modelos con Tipo de modelo "Chat (generación de texto)"). La respuesta se guarda en la caché a la vuelta.

Hay unos 20 millones de preguntas distintas, y algunas se escriben mucho más que otras (una popularidad Zipf de 0,95): la gente las elige entre las sugerencias del buscador, así que una pregunta popular llega palabra por palabra. Alrededor de una cuarta parte de las preguntas significan lo mismo que una anterior con otras palabras. Una respuesta con sus fuentes ocupa unos 4 KB. El índice vectorial guarda 100 millones de pasajes.

Una pregunta envía unos 1500 tokens al modelo de chat (2500 en el p95: la pregunta, las instrucciones y los pasajes encontrados) y recibe unos 250 (500 en el p95). El modelo de chat es la API de Claude con el nivel de tu cuenta: un Límite de peticiones por minuto de 16 000 y un Límite de tokens por minuto de 30 000 000. No puedes subir ese nivel este trimestre, y alguien espera cada respuesta, así que el Modo de API se queda en Tiempo real. Una llamada que pasa un límite se rechaza con un 429 y cuenta como petición fallida.

Dos reglas más:

- **Frescura.** Las noticias cambian las respuestas, así que una respuesta puede servirse desde una caché (una tarjeta de Caché o una CDN) durante 10 minutos como mucho: un TTL de 600 s o menos.
- **Paciencia.** Escribir una respuesta lleva segundos, así que el Tiempo límite del Cliente web es de 20 000 ms. Mantenlo.

## Las pruebas

Una prueba es visible. Puedes ejecutarla tantas veces como quieras:

- **Un día normal**: el tráfico de arriba, en el pico.

Hay dos pruebas ocultas más, y solo ves sus nombres: **Estalla una noticia: todos hacen las mismas pocas preguntas** y **El pico de la tarde dura cuatro minutos**. Tu diseño tiene que pasar las tres.
