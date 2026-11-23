# Bot de respuestas del centro de ayuda

## El producto

El centro de ayuda de tu empresa estrena un chatbot. Un cliente escribe una pregunta con sus palabras ("¿cómo cambio mi contraseña?", "olvidé mi contraseña, ¿qué hago?") y el bot la responde con un modelo de lenguaje alojado, la API de Claude. Cada pregunta enviada al modelo cuesta tokens, y tu cuenta tiene un límite de tokens por minuto. Pero la mayoría de las preguntas no son nuevas: mucha gente pregunta lo mismo con palabras algo distintas. Tu trabajo es reservar el modelo para las preguntas que de verdad tiene que responder.

## Lo que debes cumplir

| Objetivo | Meta |
| --- | --- |
| Tráfico pico | 12 peticiones/s |
| Latencia p95 | 5000 ms o menos |
| Disponibilidad | 99 % o más |
| Errores | 1 % o menos |
| Presupuesto | 80 000 $ al mes |

## El tráfico

- **Hacer una pregunta al centro de ayuda** (95 %). La petición puede pasar por una tarjeta de borde, llega a tus Servidores de aplicaciones, puede leer una caché (cache-aside) y, si falla la caché, el Servidor de aplicaciones llama al modelo y guarda la respuesta en la caché. Hay unas 100 000 formas distintas de escribir las preguntas, y algunas redacciones exactas son mucho más comunes que otras (una popularidad Zipf de 0,9). Alrededor del 60 % de estas preguntas son casi repetidas: alguien ya hizo antes una pregunta con el mismo significado.
- **Preguntar por el tema del que habla todo el mundo** (5 %). La misma ruta. En un día normal es un tema pequeño, pero los clientes que lo preguntan lo escriben de unas 50 000 formas distintas, y el 95 % significa lo mismo que una pregunta anterior.

Una pregunta envía unos 500 tokens al modelo (900 en el p95: la pregunta, las instrucciones y los artículos de ayuda) y recibe unos 120 (250 en el p95). Tu tarjeta de Inferencia de modelos es la API de Claude con sus valores por defecto: sus velocidades, un Límite de peticiones por minuto de 4000 y un Límite de tokens por minuto de 400 000. Esos límites son el nivel de tu cuenta y no puedes subirlos para este lanzamiento (el simulador los aplica a cada tarjeta de API de Claude (el Inspector los muestra, sin poder cambiarlos)), y las respuestas hacen falta mientras el cliente espera, así que el Modo de API se queda en Tiempo real. Una llamada que pasa un límite se rechaza con un 429 y cuenta como petición fallida. El cliente se rinde a los 5 s.

## Las pruebas

Una prueba es visible. Puedes ejecutarla tantas veces como quieras:

- **Un día normal**: el tráfico de arriba, en el pico.

Hay dos pruebas ocultas más, y solo ves sus nombres: **Una pregunta se hace viral: el triple de tráfico** y **El pico de la tarde, y un nodo de caché se reinicia**. Tu diseño tiene que pasar las tres.
