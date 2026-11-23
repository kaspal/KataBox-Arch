# RAG para muchos inquilinos

## El producto

Llevas un servicio de preguntas y respuestas que las empresas compran para sus propios documentos. Cada empresa es un inquilino: sus empleados hacen preguntas, y el servicio las responde a partir de los manuales, las políticas y los tickets de ese inquilino. Una única plataforma compartida atiende a todos. Un modelo de embeddings convierte la pregunta en un vector, una búsqueda vectorial encuentra los 50 pasajes de los documentos de ese inquilino más cercanos a él, y un modelo de chat escribe la respuesta a partir de ellos.

Hay unos 400 inquilinos pequeños, un inquilino grande (una cadena de tiendas) y un inquilino mediano cuyos desarrolladores llaman al servicio desde scripts. El inquilino grande está a punto de pasar a todos sus equipos al servicio, lo que traerá unas cuatro veces su tráfico de hoy. Los scripts del inquilino mediano ya han enviado avalanchas otras veces, y nada los ha frenado todavía.

El contrato de cada inquilino dice lo mismo: una clave de API puede enviar hasta 100 preguntas por segundo. Por encima de eso, el servicio puede rechazar preguntas con un HTTP 429.

## Lo que debes cumplir

| Objetivo | Valor |
| --- | --- |
| Tráfico pico | 100 preguntas/s |
| Tiempo hasta el primer token p95 | 2000 ms o menos |
| Latencia p95 (la respuesta entera) | 10 000 ms o menos |
| Disponibilidad | 99,9 % o más |
| Errores | 1 % o menos |
| Presupuesto | 15 000 $ al mes |

## El tráfico

- **Una pregunta del inquilino grande** (20 %). Una clave de API: unas 20 preguntas por segundo en el pico.
- **Una pregunta de un inquilino pequeño** (70 %). 400 inquilinos, cada uno con su propia clave de API, se reparten este tráfico a partes iguales: unas 70 preguntas por segundo en total, muy por debajo de una por segundo cada uno.
- **Una pregunta del inquilino cuyo script inunda** (10 %). Una clave de API: unas 10 preguntas por segundo en un día normal.

Todas las preguntas siguen el mismo camino. Pueden pasar por tarjetas de borde (un Balanceador de carga, un Limitador de tasa) y llegan a tus Servidores de aplicaciones. El Servidor de aplicaciones llama a tres cosas en orden y conserva su hueco hasta que la respuesta termina: una tarjeta de Inferencia de modelos con el tipo de modelo «Embeddings», una Base de datos vectorial y una tarjeta de Inferencia de modelos con el tipo de modelo «Chat (generación de texto)». El prompt (la pregunta más los pasajes encontrados) tiene unos 1500 tokens, 3000 en el p95. La respuesta tiene unos 200 tokens, 400 en el p95.

Los documentos de todos los inquilinos suman 500 millones de vectores: pon «Vectores (millones)» a 500 en tus tarjetas de Base de datos vectorial. En el simulador, una búsqueda tarda más cuantos más vectores guarda cada shard y cuantos más vecinos pide, y cada shard ejecuta 16 búsquedas a la vez. Con 500 millones de vectores en un shard y 50 vecinos, una búsqueda tarda unos 45 ms.

En el simulador, las preguntas de todos los inquilinos se reparten entre las mismas tarjetas: no puedes reservar una tarjeta para un inquilino. Lo que sí puedes hacer por inquilino es limitarlo. Un Limitador de tasa guarda un token bucket por cada clave de API: su Límite son preguntas por segundo por clave, y su Ráfaga, cuántas puede enviar una clave de golpe. Una pregunta que rechaza recibe un 429. Para el inquilino grande y los pequeños es un error; para el inquilino cuyo script inunda, un rechazo por encima de su contrato es lo que debe pasar, así que cuenta como un éxito.

El tiempo hasta el primer token cuenta desde que llega la pregunta hasta que un modelo envía su primer token. Los clientes esperan hasta 15 s una respuesta: pon el Tiempo límite del cliente web a 15 000 ms.

## Las pruebas

Hay una prueba visible. Puedes ejecutarla tantas veces como quieras:

- **Un día normal**: el tráfico descrito arriba, en su pico.

Hay dos pruebas ocultas más, de las que solo ves el nombre: **El inquilino grande trae a todos sus equipos** y **El script de un inquilino inunda el servicio**. Tu diseño tiene que superar las tres.
