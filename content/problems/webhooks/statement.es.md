# Entrega de webhooks

## El producto

Tienes una plataforma que avisa a tus clientes cuando pasa algo: se paga un pedido, un archivo está listo, alguien se registra. Cada cliente te da un endpoint HTTPS y tú le envías cada evento como un webhook. Tu Events API recibe el evento, lo deja en una cola y responde enseguida; detrás, unos Senders lo entregan. No controlas los servidores de los clientes. Algunos son lentos, otros se caen, y tu cliente más grande, Acme, recibe casi un tercio de todos los eventos. Pase lo que pase en su lado, un evento nunca debe desaparecer sin más.

## Lo que debes cumplir

| Objetivo | Valor |
| --- | --- |
| Tráfico pico | 2000 eventos/s |
| Latencia p95 hasta completar | 5 s o menos |
| Disponibilidad | 99,9 % o más |
| Presupuesto | 10 000 $ al mes |
| Mensajes perdidos | Ninguno |

## El tráfico

Todos los eventos siguen el mismo camino. La Events API, un App Server, lo publica en una cola y termina. Un Worker lo toma de la cola y llama al endpoint del cliente. La latencia se mide hasta que el endpoint acepta el evento.

- **Entregar un evento** (70 %). Va a los endpoints de los clientes en general. Responden en unos 200 ms de mediana, con una cola lenta que llega a 2 s.
- **Entregar un evento a Acme** (30 %). El endpoint de Acme responde en unos 100 ms de mediana.

Un Worker conserva su hueco durante toda la llamada, así que un endpoint lento lo ocupa todo lo que tarde en responder. Una llamada a un endpoint caído solo falla tras un tiempo de conexión agotado de 1 s.

## Las pruebas

Hay una prueba visible. Puedes ejecutarla tantas veces como quieras:

- **Un día normal**: el tráfico descrito arriba, en su pico.

Hay dos pruebas ocultas más, de las que solo ves el nombre: **El endpoint de Acme cae durante un minuto** y **Los endpoints de los clientes se vuelven lentos**. Tu diseño tiene que superar las tres.
