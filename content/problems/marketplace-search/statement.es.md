# Búsqueda en un marketplace

## El producto

Gestionas el buscador de un marketplace. Los compradores escriben lo que quieren y esperan al instante una página de anuncios que encajen. Los vendedores editan sus anuncios todo el día, cambiando precios, fotos y stock, y un comprador debería encontrar la versión nueva poco después de que se guarde. Los anuncios viven en una base de datos relacional, que es la fuente de verdad, pero una base de datos relacional busca mal, así que las búsquedas se sirven desde un índice de búsqueda que se mantiene al día con ella. Unas pocas consultas se escriben muchísimo más que el resto.

## Lo que debes cumplir

| Objetivo | Valor |
| --- | --- |
| Tráfico pico | 5000 peticiones/s |
| Lecturas | 95 % de las peticiones |
| Latencia p95 (las actualizaciones, hasta completar) | 200 ms o menos |
| Disponibilidad | 99,9 % o más |
| Presupuesto | 2500 $ al mes |
| Mensajes perdidos | Ninguno |

## El tráfico

Compradores y vendedores llegan a la Search API, un App Server, normalmente a través de un Load Balancer.

- **Buscar anuncios** (95 %). El App Server puede consultar primero una Cache y, si no, pregunta al motor de búsqueda. Hay 200 000 consultas distintas, cada respuesta ocupa unos 8 KB, y su popularidad sigue una curva Zipf 1,1: un grupo pequeño de consultas forma la mayoría de las búsquedas.
- **Actualizar un anuncio** (5 %). El App Server escribe el anuncio en la base de datos principal. Después, la base de datos publica el cambio confirmado en un stream (captura de cambios, CDC), y un Worker lo toma del stream y lo escribe en el índice de búsqueda. La latencia se mide hasta completar: hasta que el índice tiene el cambio, no solo hasta que responde el App Server.

No intervienen proveedores externos.

## Las pruebas

Hay una prueba visible. Puedes ejecutarla tantas veces como quieras:

- **Un día normal**: el tráfico descrito arriba, en su pico.

Hay dos pruebas ocultas más, de las que solo ves el nombre: **Una reindexación en hora punta** y **Todo el mundo busca lo mismo**. Tu diseño tiene que superar las tres.
