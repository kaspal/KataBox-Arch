# Analítica de uso de un LLM

## El producto

Tienes un asistente de escritura: la gente chatea con él, y tu backend envía cada mensaje a un modelo de lenguaje alojado y devuelve la respuesta. Finanzas y producto quieren saber cuánto cuesta cada conversación y lo rápido que responde el modelo, equipo por equipo. Por eso cada mensaje del chat registra también una traza: a qué modelo llamó, los tokens de entrada y de salida, cuánto tardó y cuánto costó. Las trazas van a un Almacén de datos, donde las leen los paneles de analítica. Finanzas factura a cada equipo a partir de esas trazas, así que no puede perderse ninguna: un mensaje cuya traza no se puede registrar cuenta como fallido. Los paneles, en cambio, pueden ir unos minutos por detrás. La persona del chat no puede esperarlos.

## Lo que debes cumplir

| Objetivo | Valor |
| --- | --- |
| Tráfico pico | 5 mensajes/s |
| Latencia p95 (la respuesta entera) | 12 s o menos |
| Disponibilidad | 99 % o más |
| Errores | 1 % o menos |
| Presupuesto | 160 000 $ al mes |

## El tráfico

- **Enviar un mensaje al chat** (100 %). La petición puede pasar por un balanceador de carga y llega a tus Servidores de aplicaciones, que llaman a una tarjeta de Inferencia de modelos para obtener una respuesta de chat. Un mensaje envía unos 800 tokens al modelo (1600 en el p95) y recibe unos 300 tokens (600 en el p95). Después, su traza va al Almacén de datos de una de dos formas: el Servidor de aplicaciones la escribe allí él mismo, o publica la traza en un Stream, y el consumidor del Stream (un Procesador de streams o un worker) la escribe en el almacén. El p95 se mide sobre la respuesta del chat: desde el mensaje hasta que responde el Servidor de aplicaciones.

Una respuesta tarda varios segundos en llegar entera, así que la ventana del chat (la tarjeta Cliente) la espera hasta 30 s: pon el Tiempo límite del Cliente en 30 000 ms.

Tu cuenta del modelo permite 4000 peticiones y 2 000 000 de tokens por minuto: ponlos como Límite de peticiones por minuto y Límite de tokens por minuto de la tarjeta de Inferencia de modelos. Un almacén de datos está hecho para lecturas grandes, no para escrituras pequeñas: escribir una traza ocupa un hueco del almacén unos 300 ms. Un almacén Snowflake tiene 8 huecos por instancia y solo se cobra mientras funciona.

## Las pruebas

Hay una prueba visible. Puedes ejecutarla tantas veces como quieras:

- **Un día normal**: el tráfico descrito arriba, en su pico.

Hay dos pruebas ocultas más, de las que solo ves el nombre: **Día de lanzamiento: 4 veces el tráfico durante 2 minutos** y **El almacén de datos se ralentiza: cada escritura tarda 8 s más durante 40 s**. Un lanzamiento real dura horas; la prueba dura 2 minutos. Tu diseño tiene que superar las tres.
