# Resultados deportivos en directo

## El producto

Tienes una app de resultados en directo. En un día de partido, millones de aficionados tienen la app abierta, y cada uno mantiene una conexión WebSocket con tu plataforma. Cuando un equipo marca, la actualización baja por esa conexión y el aficionado la ve en menos de un segundo. Los aficionados también abren páginas de partido: las alineaciones, las estadísticas y el marcador hasta el momento. Un aficionado desconectado no ve nada, así que casi todos tienen que seguir conectados todo el tiempo, incluso cuando falla una parte de tu plataforma.

## Lo que debes cumplir

| Objetivo | Valor |
| --- | --- |
| Tráfico pico | 2000 cargas de página/s y 5 000 000 de conexiones abiertas |
| Latencia p95 | 1 s o menos |
| Tasa de errores | 1 % o menos |
| Disponibilidad | 99,9 % o más |
| Aficionados conectados | 95 % o más en cada segundo |
| Presupuesto | 1800 $ al mes |

## El tráfico

Hay dos tipos de petición:

- **Aficionados siguiendo un partido**. 5 000 000 de aficionados mantienen una conexión WebSocket abierta, a través de un balanceador de carga hasta un gateway de WebSocket. Todos están conectados desde el principio. Cada aficionado recibe una actualización del marcador de media cada 50 segundos, así que la plataforma envía 100 000 mensajes por segundo en total. Para un aficionado, la latencia es el tiempo que tarda en llegarle una actualización. Un intento de conexión rechazado cuenta como error.
- **Abrir la página de un partido** (100 % del tráfico de páginas, 2000 rps). La petición pasa por un borde opcional hasta un servidor de aplicaciones, y después por una caché opcional y una base de datos. Hay 2000 partidos, y unos pocos grandes se llevan la mayoría de las visitas (Zipf 1,0).

En este problema no hay servicios externos.

## Las pruebas

Hay una prueba visible. Puedes ejecutarla tantas veces como quieras:

- **Una jornada con mucho partido**: el tráfico descrito arriba, en su pico.

Hay dos pruebas ocultas más, de las que solo ves el nombre: **Un gol en el último minuto** y **Cae una pasarela en el descanso**. Tu diseño tiene que superar las tres.
