# Failover entre proveedores de chat

## El producto

Tienes el asistente de soporte dentro de una app de compras. Un cliente escribe una pregunta, tu backend la envía a un modelo de lenguaje alojado junto con algo de contexto sobre su pedido, y la respuesta llega al chat a medida que se genera. El modelo se lo compras a un proveedor externo, y los proveedores tienen días malos: a veces la API está caída, y a veces acepta las llamadas y luego nunca responde. Un cliente que ve un indicador de carga durante un minuto se rinde y llama al teléfono de soporte, así que el asistente debería seguir respondiendo aunque falle un proveedor. También tienes una cuenta con un segundo proveedor.

## Lo que debes cumplir

| Objetivo | Valor |
| --- | --- |
| Tráfico pico | 20 mensajes/s |
| Latencia p95 (respuesta completa) | 6000 ms o menos |
| Tiempo hasta el primer token p95 | 1500 ms o menos |
| Disponibilidad | 99 % o más |
| Errores | 1 % o menos |
| Presupuesto | 250 000 $ al mes |

## El tráfico

- **Enviar un mensaje de chat** (100 %). El mensaje puede pasar por un balanceador de carga y llega a tus Servidores de aplicaciones. Cada Servidor de aplicaciones llama a un modelo de chat (una tarjeta de Inferencia de modelos con el Tipo de modelo "Chat (generación de texto)") y espera la respuesta completa. Un prompt tiene unos 600 tokens (hasta unos 1500), y una respuesta unos 60 tokens (hasta unos 150).

La app de chat espera hasta 30 s una respuesta: pon el Tiempo límite del Cliente web en 30 000 ms.

Tus dos cuentas de proveedor tienen estos límites por minuto. El simulador los aplica a cada tarjeta de ese proveedor (el Inspector los muestra, sin poder cambiarlos):

| Tarjeta | Límite de peticiones por minuto | Límite de tokens por minuto |
| --- | --- | --- |
| API de Claude | 4000 | 2 000 000 |
| API de OpenAI | 1500 | 1 000 000 |

Una llamada por encima de un límite se rechaza al instante (un HTTP 429). Cuando un Servidor de aplicaciones está conectado a más de un modelo de chat, les envía sus llamadas por turnos. Las comprobaciones de salud detectan en unos 4 s una tarjeta que deja de responder, y desde entonces los Servidores de aplicaciones la saltan; una tarjeta que responde despacio sigue pareciendo sana.

## Las pruebas

Hay una prueba visible. Puedes ejecutarla tantas veces como quieras:

- **Un día normal**: el tráfico descrito arriba, en su pico.

Hay dos pruebas ocultas más, de las que solo ves el nombre: **Un proveedor de modelos cae durante 60 s** y **Un proveedor de modelos se cuelga durante 60 s**. En las dos, el proveedor que falla puede ser cualquiera de los dos. Tu diseño tiene que superar las tres.
