# Campañas de correo masivo

## El producto

Tienes una plataforma de marketing. Tus clientes escriben una campaña, eligen el público y pulsan enviar, y tu plataforma entrega los correos, uno por destinatario. No tienes servidores de correo propios: cada correo sale a través de un proveedor de correo externo, y ese proveedor decide a qué velocidad puedes enviar. Que una campaña llegue un poco más tarde de lo previsto no le importa a nadie. Lo que sí le importa a un cliente es un destinatario que nunca recibe el correo, así que no se puede perder ninguno.

## Lo que debes cumplir

| Objetivo | Valor |
| --- | --- |
| Tráfico pico | 500 correos/s |
| Latencia p95 hasta completar | 30 s o menos |
| Disponibilidad | 99,9 % o más |
| Presupuesto | 3300 $ al mes |
| Mensajes perdidos | Ninguno |

## El tráfico

Aquí no hay una API para usuarios. Un Scheduler se dispara cada segundo y publica en una cola los correos de ese segundo.

- **Enviar un correo de campaña** (100 %). Un Worker toma el correo de la cola y llama al proveedor de correo. La latencia se mide desde que se publica el correo hasta que el proveedor lo acepta.

El proveedor responde en unos 150 ms de mediana, con una cola lenta que llega a 1,2 s. Además tiene un límite de ritmo: te acepta como mucho 800 llamadas por segundo. Las llamadas por encima del límite se rechazan (un HTTP 429) y cuentan como fallidas, así que el correo tiene que volver a intentarse más tarde o se pierde. Un Worker conserva su hueco durante toda la llamada.

## Las pruebas

Hay una prueba visible. Puedes ejecutarla tantas veces como quieras:

- **Una campaña normal**: el tráfico descrito arriba, en su pico.

Hay dos pruebas ocultas más, de las que solo ves el nombre: **El proveedor te baja el límite** y **Se solapan dos campañas**. Tu diseño tiene que superar las tres.
