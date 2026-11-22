# Chat

## El producto

Tienes un servicio de chat. La gente deja la app abierta toda la tarde y envía mensajes cortos a amigos y grupos. Cada teléfono mantiene una conexión WebSocket de larga duración con uno de tus servidores gateway. Un mensaje llega al gateway de quien lo envía, pero la persona a la que va dirigido puede estar conectada a otro distinto. Aun así, tiene que llegarle rápido. Cuando una conversación se queda parada a mitad de frase, la gente lo nota enseguida.

## Lo que debes cumplir

| Objetivo | Valor |
| --- | --- |
| Tráfico pico | 4 000 mensajes/s |
| Latencia p95 de entrega | 150 ms o menos |
| Disponibilidad | 99,9 % o más |
| Presupuesto | 12 000 $ al mes |

En este problema, las peticiones de pub/sub son la mayor parte de la factura. Tenlas en cuenta al planificar.

## El tráfico

- **Enviar un mensaje** (100 %). El mensaje viaja por un WebSocket a través de un componente de borde hasta un gateway, que es un App Server. El gateway publica el mensaje en un broker y se desentiende de él. El gateway que tiene la conexión del destinatario lo consume y lo entrega. La latencia se mide desde que se envía el mensaje hasta que se entrega, no solo hasta que el primer gateway lo acepta.

Todos los mensajes siguen este camino, así que aquí no hay un reparto entre lecturas y escrituras en el que apoyarse. Lo que importa es cómo pasan los mensajes de un gateway a otro, y qué ocurre cuando falta un gateway.

## Las pruebas

Hay una prueba visible. Puedes ejecutarla tantas veces como quieras:

- **Una tarde normal**: el tráfico descrito arriba, en su pico.

Hay dos pruebas ocultas más, de las que solo ves el nombre: **Se cae un gateway** y **Pico de la noche**. Tu diseño tiene que superar las tres.
