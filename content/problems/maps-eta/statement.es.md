# Mapas y horas de llegada

## El producto

Tienes una app de mapas. La gente se mueve por el mapa, que se carga mosaico a mosaico, y pide una ruta desde donde está hasta donde va, con una hora de llegada. La ruta sale de tu propio grafo de carreteras. La hora de llegada es mejor con el tráfico en vivo, que compras a un feed de tráfico externo. Una ruta con una hora de llegada calculada con velocidades típicas sigue siendo útil; una ruta que nunca llega, no.

## Lo que debes cumplir

| Objetivo | Valor |
| --- | --- |
| Tráfico pico | 4000 rps |
| Latencia p95 | 250 ms o menos |
| Disponibilidad | 99,9 % o más |
| Errores | 0,5 % o menos |
| Presupuesto | 9000 $ al mes |

## El tráfico

- **Cargar un mosaico del mapa** (70 %). El mosaico se lee del almacenamiento de objetos, a través de una CDN si la hay. Hay 2 000 000 de mosaicos, y unos pocos (las ciudades con más movimiento) se piden mucho más que el resto. Un mosaico se puede cachear.
- **Planificar una ruta con hora de llegada** (30 %). Un servidor de aplicaciones recorre el grafo de carreteras en una base de datos de grafos: 6 saltos, con 3 carreteras que salen de cada cruce, unas 1100 relaciones por ruta. Después pide al feed de tráfico en vivo que ajuste la hora de llegada.

El feed de tráfico responde en unos 40 ms de mediana, con una cola lenta de hasta 200 ms. La llamada es opcional: si falla o se agota el tiempo, la ruta vuelve igualmente, con una hora de llegada calculada con velocidades típicas.

## Las pruebas

Hay una prueba visible. Puedes ejecutarla tantas veces como quieras:

- **Una tarde normal**: el tráfico descrito arriba, en su pico.

Hay dos pruebas ocultas más, de las que solo ves el nombre: **Hora punta** y **Cae el feed de tráfico**. Tu diseño tiene que superar las tres.
