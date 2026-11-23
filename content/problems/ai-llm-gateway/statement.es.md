# Pasarela de LLM para muchos equipos

## El producto

Llevas el equipo de plataforma de una empresa en la que diez equipos de producto construyen funciones sobre un modelo de lenguaje grande. Nadie tiene su propia clave del proveedor: cada equipo llama al modelo a través de una pasarela LiteLLM compartida, con su propia clave de API, y la pasarela reenvía las llamadas a una única cuenta de la empresa en la API de Claude. Esa cuenta tiene un límite fijo de 4000 peticiones y 400 000 tokens por minuto. Cuando la cuenta lo supera, el proveedor rechaza las llamadas con un HTTP 429, y le da igual qué equipo gastó los tokens: se rechazan las llamadas de todos.

Cada equipo ha acordado una cuota de 55 000 tokens por minuto. Uno de los equipos lanza a veces un script por lotes contra la pasarela, y nada lo ha frenado todavía.

## Lo que debes cumplir

| Objetivo | Valor |
| --- | --- |
| Tráfico pico | 8 peticiones/s |
| Latencia p95 (la respuesta entera) | 5000 ms o menos |
| Tiempo hasta el primer token p95 | 1500 ms o menos |
| Disponibilidad | 99,9 % o más |
| Errores | 1 % o menos |
| Presupuesto | 80 000 $ al mes |

## El tráfico

- **Una petición de chat de un equipo** (90 %). Nueve equipos, cada uno con su propia clave de API, se reparten este tráfico a partes iguales.
- **Llamadas del equipo que inunda** (10 %). El décimo equipo, con una clave de API. En un día normal envía lo mismo que cualquier otro equipo.

Una petición típica envía unos 400 tokens de prompt (1000 en el p95) y recibe unos 60 tokens (120 en el p95); las largas suben un poco la media. Cada petición pasa por un Balanceador de carga y la API Gateway (y por cualquier Limitador de tasa que pongas en el camino), y luego un Servidor de aplicaciones llama a la tarjeta de Inferencia de modelos y espera la respuesta entera. En el simulador, una tarjeta de pasarela deja pasar las peticiones pero no puede llamar a un modelo, así que el Servidor de aplicaciones representa al proceso de la pasarela que hace la llamada al proveedor.

El "Límite de peticiones por minuto" y el "Límite de tokens por minuto" de la tarjeta de Inferencia de modelos son los límites de tu cuenta con el proveedor, 4000 y 400 000: el simulador los aplica a cada tarjeta de API de Claude (el Inspector los muestra, sin poder cambiarlos).

Un Limitador de tasa guarda un token bucket por cada clave de API. Su Cuenta dice qué cuenta el bucket: Peticiones, o Tokens (prompt + respuesta). Con Tokens, cada petición saca del bucket sus tokens de prompt y de respuesta, el Límite son tokens por segundo por clave, y la Ráfaga, cuántos tokens puede gastar una clave de golpe. Una petición que rechaza recibe un 429. Para los nueve equipos normales es un error; para el equipo que inunda, un rechazo es justo lo que debe pasar, así que cuenta como un éxito.

## Las pruebas

Hay una prueba visible. Puedes ejecutarla tantas veces como quieras:

- **Un día normal**: el tráfico descrito arriba, en su pico.

Hay dos pruebas ocultas más, de las que solo ves el nombre: **Un equipo lanza un script que inunda la pasarela** y **Se pierde un nodo de la pasarela**. Tu diseño tiene que superar las tres.
