## 1

En un día normal, una GPU parece casi ociosa. ¿Con cuántas publicaciones trabaja el clasificador a la vez en el pico normal, y con cuántas cuando una publicación viral trae cinco veces el tráfico? Y si ese servidor de GPU se para, ¿quién comprueba la siguiente publicación?

## 2

Publicaciones en curso = publicaciones por segundo × tiempo por publicación. Con 1500 publicaciones por segundo y unos 13 ms cada una, son unas 20 a la vez; con cinco veces el tráfico son unas 98, más que los 64 Huecos de lote por GPU de una instancia. Dimensiona el nivel del modelo para el pico, no para la media. Y pon el clasificador en dos servidores de modelo separados, cada uno capaz de llevar toda la carga, para que perder uno cueste un momento y no el producto.

## 3

Pon un AWS ALB delante de 4 Go App Servers. Conéctalos a dos tarjetas de Inferencia de modelos NVIDIA Triton, cada una con 2 Instancias y el Tipo de modelo en Clasificador. Los App Servers reparten las llamadas entre las dos, y cualquiera de ellas sola puede llevar el pico viral.
