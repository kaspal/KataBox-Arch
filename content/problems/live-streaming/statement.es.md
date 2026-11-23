# Directos con chat

## El producto

Tienes una web de directos. Los streamers emiten en directo y los espectadores los ven en el navegador. El reproductor no hace una única descarga larga del vídeo: cada pocos segundos descarga el siguiente segmento corto de la emisión. Junto al vídeo está el chat del canal, donde los espectadores envían mensajes y todos los que están en el chat los ven llegar por una conexión WebSocket abierta. Unos pocos canales grandes reúnen a la mayor parte del público, y cuando un streamer manda a sus espectadores a otro canal (una raid), el chat de ese canal se inunda en un instante.

## Lo que debes cumplir

| Objetivo | Valor |
| --- | --- |
| Tráfico pico | 5000 rps y 200 000 conexiones de chat abiertas |
| Latencia p95 | 300 ms o menos |
| Tasa de errores | 1 % o menos |
| Disponibilidad | 99,9 % o más |
| Espectadores conectados al chat | 95 % o más en cada segundo |
| Presupuesto | 15 000 $ al mes |

## El tráfico

Hay tres tipos de petición:

- **Descargar el siguiente segmento de vídeo** (95 %, 4750 rps). El reproductor descarga un segmento del almacenamiento de objetos, a través de un borde opcional delante de él. En cada momento hay 3000 segmentos en directo, y los canales grandes se llevan la mayoría de las descargas (Zipf 1,2).
- **Enviar un mensaje al chat** (5 %, 250 rps). El mensaje pasa por un borde opcional hasta un servidor de aplicaciones, que lo escribe en una base de datos.
- **Espectadores en el chat**. 200 000 espectadores mantienen una conexión WebSocket abierta, a través de un balanceador de carga hasta un gateway de WebSocket. Todos están conectados desde el principio. Cada espectador recibe de media 0,5 mensajes de chat por segundo, así que la plataforma envía 100 000 mensajes por segundo en total. Para un espectador, la latencia es el tiempo que tarda en llegarle un mensaje. Un intento de conexión rechazado cuenta como error.

En este problema no hay servicios externos.

## Las pruebas

Hay una prueba visible. Puedes ejecutarla tantas veces como quieras:

- **Una tarde normal**: el tráfico descrito arriba, en su pico.

Hay dos pruebas ocultas más, de las que solo ves el nombre: **Una raid inunda un canal** y **El almacenamiento de origen se vuelve lento**. Tu diseño tiene que superar las tres.
