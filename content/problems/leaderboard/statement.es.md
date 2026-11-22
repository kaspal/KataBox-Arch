# Clasificación de un juego

## El producto

Un juego para móviles tiene 50 clasificaciones, una por modo y región. La pantalla del juego muestra los 100 mejores jugadores de una clasificación, y los jugadores la abren a menudo para ver en qué puesto están. Cuando termina una partida, el juego envía la puntuación del jugador.

Algunas clasificaciones están calientes: el modo principal en las regiones más grandes se lleva la mayoría de las visitas. El resto está tranquilo.

## Lo que debes cumplir

| Requisito | Objetivo |
| --- | --- |
| Tráfico pico | 8 000 solicitudes/s |
| Lecturas | 80 % de las solicitudes |
| Latencia p95 | 50 ms o menos |
| Disponibilidad | 99,9 % o más |
| Presupuesto | 1 500 $/mes o menos |

## El tráfico

- **Leer los 100 primeros** (80 %). Devuelve las 100 mejores puntuaciones de una clasificación, un resultado de 32 KB. Las solicitudes se reparten entre las 50 clasificaciones con un fuerte sesgo hacia unas pocas. Un App Server la lee del almacén: un Cache usado como almacén o una base de datos.
- **Enviar una puntuación** (20 %). Un App Server escribe la nueva puntuación en ese mismo almacén.

No hay otra fuente de verdad. Lo que elijas como almacén guarda las clasificaciones.

## Las pruebas

Tu diseño se evalúa con una prueba visible y dos pruebas ocultas.

- **Un día normal** (visible). Un día estable con el tráfico pico.
- **Termina el torneo** (oculta).
- **El almacén se reinicia** (oculta).

Los errores deben quedar por debajo del 0,1 %.
