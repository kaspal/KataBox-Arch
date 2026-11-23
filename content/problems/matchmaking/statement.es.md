# Emparejamiento de partidas

## El producto

Llevas el emparejamiento de un juego en línea. Un jugador pulsa «Buscar partida» y tu servicio lo pone en una cola. Un emparejador toma jugadores de la cola, forma una partida y la escribe en la base de datos, y entonces la partida puede empezar. Tus jugadores están en Norteamérica y en Europa. Nadie espera una partida al instante, pero un jugador que espera varios minutos, o cuya petición desaparece, cierra el juego.

## Lo que debes cumplir

| Objetivo | Valor |
| --- | --- |
| Tráfico pico | 1000 peticiones/s |
| Latencia p95 hasta completar | 5 s o menos |
| Disponibilidad | 99,9 % o más |
| Errores | 1 % o menos |
| Presupuesto | 3500 $ al mes |
| Peticiones perdidas | Ninguna |

## El tráfico

- **Buscar partida** (100 %). La petición pasa por un App Server, que pone al jugador en una cola y responde enseguida. Un Worker emparejador toma al jugador de la cola, forma la partida (unos 200 ms de trabajo) y la escribe en una base de datos NoSQL. La latencia se mide hasta completar: desde la petición hasta que la partida está escrita.

En este problema no hay servicios externos.

## Las pruebas

Hay una prueba visible. Puedes ejecutarla tantas veces como quieras:

- **Una tarde entre semana**: el tráfico descrito arriba, en su pico.

Hay dos pruebas ocultas más, de las que solo ves el nombre: **Sábado por la noche** y **Se apaga una región**. Tu diseño tiene que superar las tres.
