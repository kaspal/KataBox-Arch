# Moderación de publicaciones

## El producto

Tienes una app social. Antes de que una publicación nueva salga a la luz, un clasificador de moderación lee su texto y la marca como correcta o como contraria a las normas. Nunca se publica nada sin revisar: si la comprobación falla, el usuario ve un error y tiene que volver a intentarlo. Alojas tú el clasificador en NVIDIA Triton, en tus propias GPU. La mayoría de los días el tráfico es estable, pero cuando una publicación se hace viral, llega una avalancha de respuestas y republicaciones en cuestión de segundos.

## Lo que debes cumplir

| Objetivo | Valor |
| --- | --- |
| Tráfico pico | 1500 publicaciones/s |
| Latencia p95 | 200 ms o menos |
| Disponibilidad | 99,9 % o más |
| Tasa de errores | 1 % o menos |
| Presupuesto | 5000 $ al mes |

## El tráfico

- **Publicar un mensaje** (100 %). La petición puede pasar por un balanceador de carga, llega a tus App Servers y estos llaman al clasificador antes de aceptar la publicación. El clasificador recibe unos 40 tokens de texto (hasta unos 90) y devuelve una etiqueta. En Triton necesita unos 13 ms para una publicación típica, y cada instancia de GPU trabaja con hasta 64 publicaciones a la vez (sus Huecos de lote por GPU). Un App Server conserva su hueco durante toda la llamada.

El paso del modelo es obligatorio: una publicación que no se puede comprobar no se publica, y la petición falla.

## Las pruebas

Hay una prueba visible. Puedes ejecutarla tantas veces como quieras:

- **Un día normal**: el tráfico descrito arriba, en su pico.

Hay dos pruebas ocultas más, de las que solo ves el nombre: **Una publicación se hace viral** y **Se pierde un servidor del modelo**. Tu diseño tiene que superar las tres.
