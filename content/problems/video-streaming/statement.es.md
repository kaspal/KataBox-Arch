# Streaming de vídeo

## El producto

Tienes un servicio de vídeo. Los espectadores pulsan reproducir y el reproductor descarga el vídeo en segmentos cortos, de unos pocos segundos cada uno, uno detrás de otro. Si un segmento llega tarde, la imagen se congela y el espectador lo nota al instante. Unas pocas series se ven mucho más que el resto. Por el otro lado, los creadores suben vídeos nuevos, que hay que convertir en varias versiones (tamaños y tasas de bits) antes de que alguien pueda verlos. Las subidas son pocas comparadas con las reproducciones, pero cada una supone mucho trabajo.

## Lo que debes cumplir

| Objetivo | Valor |
| --- | --- |
| Tráfico pico | 4 000 peticiones/s |
| Reproducción de segmentos | 99 % de las peticiones |
| Latencia p95 | 80 ms o menos |
| Disponibilidad | 99,9 % o más |
| Presupuesto | 22 000 $ al mes |

## El tráfico

- **Reproducir un segmento de vídeo** (99 %). La petición puede pasar por un componente de borde, como una CDN, y luego la sirve un origen: un App Server u Object Storage. Hay 500 000 segmentos, su popularidad sigue una curva Zipf 1,1 y todos se pueden cachear.
- **Subir un vídeo** (1 %). La petición puede pasar por un componente de borde y luego llega a la Upload API, en un App Server. La Upload API guarda el vídeo y después lo publica en una Queue. Los Transcoder Workers lo recogen, dedican unos 400 ms a procesarlo y escriben las versiones.

## Las pruebas

Hay una prueba visible. Puedes ejecutarla tantas veces como quieras:

- **Una tarde normal**: el tráfico descrito arriba, en su pico.

Hay dos pruebas ocultas más, de las que solo ves el nombre: **El estreno de una temporada** y **Todos suben a la vez**. Tu diseño tiene que superar las tres.
