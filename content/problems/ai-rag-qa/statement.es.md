# Pregunta a la documentación (RAG)

## El producto

Llevas el asistente de ayuda de una gran plataforma para desarrolladores. Un usuario escribe una pregunta y el asistente la responde a partir de la documentación de la plataforma, sus hilos del foro y sus tickets de soporte: mil millones de pasajes cortos en total. Cada pregunta pasa por tres pasos. Un modelo de embeddings convierte la pregunta en un vector. Una búsqueda vectorial encuentra los 100 pasajes más cercanos. Un modelo de chat lee la pregunta y esos pasajes y escribe la respuesta, que llega palabra a palabra. Los usuarios juzgan el asistente por lo pronto que aparecen las primeras palabras, así que el tiempo hasta el primer token importa más que el tiempo hasta el último.

## Lo que debes cumplir

| Objetivo | Valor |
| --- | --- |
| Tráfico pico | 150 preguntas/s |
| Tiempo hasta el primer token, p95 | 2000 ms o menos |
| Latencia p95 (la respuesta entera) | 8000 ms o menos |
| Disponibilidad | 99,9 % o más |
| Errores | 1 % o menos |
| Presupuesto | 20 000 $ al mes |

## El tráfico

- **Hacer una pregunta** (100 %). La petición puede pasar por un balanceador de carga y llega a tus App Servers. El App Server llama a tres cosas en orden y conserva su hueco hasta que la respuesta termina: una tarjeta de Inferencia de modelos con el tipo de modelo «Embeddings», una Base de datos vectorial y una tarjeta de Inferencia de modelos con el tipo de modelo «Chat (generación de texto)». El prompt (la pregunta más los pasajes encontrados) tiene unos 1200 tokens de media, 2000 en el p95. La respuesta tiene unos 150 tokens de media, 300 en el p95.

El índice guarda mil millones de vectores: pon «Vectores (millones)» en 1000 en tus tarjetas de Base de datos vectorial. Cada búsqueda pide los 100 vecinos más cercanos. En el simulador, una búsqueda tarda más cuantos más vectores guarda cada shard y cuantos más vecinos pide, y cada shard ejecuta 16 búsquedas a la vez. Con 1000 millones de vectores en un solo shard, una búsqueda tarda unos 70 ms.

El tiempo hasta el primer token cuenta desde que llega la pregunta hasta que el modelo de chat envía su primer token, así que incluye el embedding, la búsqueda vectorial y cualquier espera en una cola por el camino. Los clientes esperan una respuesta hasta 15 s.

## Las pruebas

Hay una prueba visible. Puedes ejecutarla tantas veces como quieras:

- **Un día normal**: el tráfico descrito arriba, en su pico.

Hay dos pruebas ocultas más, de las que solo ves el nombre: **Las preguntas se duplican durante 30 s** y **Se pierde un nodo de búsqueda vectorial el día del lanzamiento**. Tu diseño tiene que superar las tres.
