# Sitio estático + CDN

## El producto

Una empresa tiene un sitio de marketing. Está hecho de páginas, imágenes y scripts que solo cambian cuando el equipo despliega una versión nueva. Hay una sola parte dinámica: un formulario de contacto que los visitantes rellenan y que debe guardarse en una base de datos.

Casi todos los visitantes solo leen. Abren la página de inicio, se desplazan, miran algunas páginas de producto y se van. Unos pocos envían el formulario.

## Lo que debes cumplir

| Requisito | Objetivo |
| --- | --- |
| Tráfico pico | 1 500 solicitudes/s |
| Archivos estáticos | 99 % de las solicitudes |
| Latencia p95 | 60 ms o menos |
| Disponibilidad | 99,9 % o más |
| Presupuesto | 4 000 $/mes o menos |

## El tráfico

- **Página, imagen o script** (99 %). El sitio tiene 20 000 archivos. Unos pocos, como la página de inicio y el script principal, se piden mucho más que el resto (una popularidad Zipf 1.0). Estas respuestas se pueden guardar en caché. Las puede servir una CDN o un origen: un App Server o un Object Storage.
- **Formulario de contacto** (1 %). Un App Server recibe el formulario y lo escribe en la base de datos.

## Las pruebas

Tu diseño se evalúa con una prueba visible y dos pruebas ocultas.

- **Un día normal** (visible). Un día estable con el tráfico pico.
- **Un despliegue purga la CDN** (oculta).
- **El origen se vuelve lento** (oculta).

Todas las pruebas deben mantener los errores por debajo del 0,1 %.
