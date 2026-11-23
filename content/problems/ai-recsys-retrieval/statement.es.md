# Recomendaciones: recuperar y reordenar

## El producto

Llevas la franja «también te puede gustar» de las páginas de artículo de una gran tienda en línea. El catálogo tiene 5 millones de artículos, y cada uno tiene un vector de embedding en un índice vectorial. Cuando un comprador abre la página de un artículo, tu servicio busca los 50 artículos cuyos vectores están más cerca del vector de ese artículo (recuperación), luego envía esos 50 candidatos a un modelo clasificador pequeño que los puntúa para esta tienda, y los devuelve en el nuevo orden (reordenación). La lista terminada de un artículo ocupa unos 2 KB, y es la misma para todos los compradores que abren ese artículo.

## Lo que debes cumplir

| Objetivo | Valor |
| --- | --- |
| Tráfico pico | 600 peticiones/s |
| Latencia p95 | 150 ms o menos |
| Disponibilidad | 99,9 % o más |
| Errores | 1 % o menos |
| Presupuesto | 3000 $ al mes |

## El tráfico

- **Mostrar «también te puede gustar» para un artículo** (100 %). La petición puede pasar por un balanceador de carga y llega a tus App Servers. El App Server puede buscar primero el artículo en una Caché (cache-aside: un acierto devuelve la lista guardada y termina la petición; un fallo sigue adelante, y la lista terminada llena la caché). Si falla, llama a una Base de datos vectorial y después a una tarjeta de Inferencia de modelos con el tipo de modelo «Clasificador». La entrada del clasificador (el artículo y sus 50 candidatos) tiene unos 50 tokens de media, 100 en el p95, y responde con un token.

La clave de la petición es el artículo que se está viendo. La popularidad de los artículos es una cola larga: unos pocos artículos se ven mucho, pero la mayoría de los 5 millones se ven poco, así que la mayoría de las peticiones son de artículos que nadie más ha pedido en los últimos minutos.

Pon «Vectores (millones)» en 5 en tus tarjetas de Base de datos vectorial. Cada búsqueda pide los 50 vecinos más cercanos. En el simulador, una búsqueda tarda más cuantos más vectores guarda cada shard y cuantos más vecinos pide; cada shard ejecuta 16 búsquedas a la vez, y las búsquedas de un mismo artículo van siempre al mismo shard. Con 5 millones de vectores en un solo shard, una búsqueda tarda unos 15 ms.

Las cachés empiezan calientes.

## Las pruebas

Hay una prueba visible. Puedes ejecutarla tantas veces como quieras:

- **Un día normal**: el tráfico descrito arriba, en su pico.

Hay dos pruebas más ocultas, y solo ves sus nombres: **Una rebaja triplica el tráfico** y **Un artículo se hace viral**. Tu diseño tiene que pasar las tres.
