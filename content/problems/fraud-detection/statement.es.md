# Detección de fraude en pagos

## El producto

Te encargas de la comprobación de fraude de una red de pagos. Los servidores de socios, los sistemas de pago de tiendas y bancos, te piden que puntúes cada pago con tarjeta antes de aprobarlo, y además te envían un flujo de eventos de tarjeta. Un modelo de fraude mira las características recientes de la tarjeta y devuelve una puntuación. La comprobación está en mitad del pago, así que tiene que ser rápida: a un cliente que espera en la caja no le importa por qué llega tarde la respuesta. Si el modelo no puede contestar a tiempo, decide en su lugar un conjunto sencillo de reglas.

## Lo que debes cumplir

| Objetivo | Valor |
| --- | --- |
| Tráfico pico | 3000 solicitudes/s |
| Latencia p95 | 60 ms o menos |
| Disponibilidad | 99,9 % o más |
| Tasa de errores | 0,5 % o menos |
| Presupuesto | 3900 $ al mes |

## El tráfico

Cada solicitud llega desde un servidor de socio y puede pasar por un balanceador de carga antes de llegar a tu API.

- **Puntuar un pago** (60 %). La API lee las características de la tarjeta de una caché y después pide una puntuación al modelo de fraude. El modelo es un clasificador: recibe unos 20 tokens de características (hasta unos 40) y necesita unos 20 ms por puntuación. El paso del modelo es opcional: si la llamada falla o tarda demasiado, el pago se comprueba con reglas y la solicitud sigue teniendo éxito.
- **Registrar un evento de tarjeta** (40 %). La API publica el evento en un stream. Un procesador de streams lo lee, lo agrega y actualiza en la caché las características de esa tarjeta, para que la siguiente puntuación las vea.

Hay unos 5 millones de tarjetas, y cada solicitud trata de una de ellas.

## Las pruebas

Hay una prueba visible. Puedes ejecutarla tantas veces como quieras:

- **Un día normal**: el tráfico descrito arriba, en su pico.

Hay dos pruebas ocultas más, de las que solo ves el nombre: **El servidor del modelo se vuelve lento** y **Black Friday**. Tu diseño tiene que superar las tres.
