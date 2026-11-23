# Hilos de comentarios

## El producto

Llevas los comentarios bajo los artículos de un sitio de noticias con mucho tráfico. Cada artículo tiene un hilo, y cada lector que baja hasta el final lo carga. La mayoría de los lectores solo leen; unos pocos publican un comentario. Hay 100 000 hilos, pero casi todas las tardes los lectores se reparten entre muchos de ellos. Algunas tardes una noticia despega, y todo el mundo quiere el mismo hilo a la vez. Los lectores aceptan que un comentario aparezca un momento tarde. Lo que no aceptan es un hilo que no carga.

## Lo que debes cumplir

| Objetivo | Valor |
| --- | --- |
| Tráfico pico | 6000 peticiones/s |
| Latencia p95 | 60 ms o menos |
| Disponibilidad | 99,9 % o más |
| Errores | 1 % o menos |
| Presupuesto | 1200 $ al mes |

## El tráfico

- **Cargar los comentarios de un hilo** (96 %). La petición puede pasar por un balanceador de carga, llega a tus App Servers y estos leen el hilo de una caché si la hay (cache-aside), y del almacén de datos en un fallo de caché. Un hilo se lee como una sola clave.
- **Publicar un comentario** (4 %). La petición puede pasar por un balanceador de carga, llega a tus App Servers y estos escriben el comentario en el almacén de datos. Si hay caché, el App Server refresca después la entrada de ese hilo en ella.

Los dos tipos de petición eligen un hilo entre 100 000, y unos pocos hilos populares se llevan la mayoría (una popularidad Zipf de 0,9). En este problema no hay servicios externos.

## Las pruebas

Hay una prueba visible. Puedes ejecutarla tantas veces como quieras:

- **Una tarde normal**: el tráfico descrito arriba, en su pico.

Hay dos pruebas ocultas más, de las que solo ves el nombre: **Un hilo se hace viral** y **Se vacía la caché en una tarde con mucho tráfico**. Tu diseño tiene que superar las tres.
