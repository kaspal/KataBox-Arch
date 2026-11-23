# Solución comentada

## Adónde va la carga

20 mensajes por segundo son 1200 por minuto. Un mensaje lleva unos 600 tokens de prompt y 60 de respuesta, así que en el pico el asistente usa unos 1200 × 660 ≈ 792 000 tokens por minuto. Un Servidor de aplicaciones conectado a dos modelos de chat les envía sus llamadas por turnos, así que en un día normal cada proveedor recibe unas 600 peticiones y 396 000 tokens por minuto, muy dentro de las dos cuentas.

Una respuesta tarda segundos, no milisegundos. En la API de Claude el primer token llega tras unos 600 ms más 600 tokens de prompt a 5000 por segundo, unos 0,72 s en total, y los 60 tokens de respuesta tardan otros 60 / 80 por segundo ≈ 0,75 s. Una respuesta larga de 150 tokens tras un prompt de 1500 tokens tarda unos 2,8 s, y las raras muy largas unos 5 s.

Cuando falla un proveedor, el otro tiene que llevarlo todo: 1200 peticiones y unos 792 000 tokens por minuto. La cuenta de OpenAI admite 1500 y 1 000 000, así que puede, con un 20 % de margen más o menos. Es lo primero que hay que comprobar en cualquier plan de failover: el respaldo tiene que aguantar toda la carga dentro de sus propios límites, no solo su mitad habitual.

Las dos pruebas ocultas hacen fallar a un proveedor de dos formas distintas. "Cae" significa que la tarjeta deja de responder: las llamadas fallan enseguida, y las comprobaciones de salud lo detectan en unos 4 s. "Se cuelga" significa que la tarjeta acepta cada llamada y responde 60 s después: nada falla, las comprobaciones de salud ven una tarjeta viva, y cada mensaje enviado allí simplemente espera.

## El diseño de referencia

El Cliente web tiene un Tiempo límite de 30 000 ms. Un AWS ALB envía los mensajes a 2 Servidores de aplicaciones Go con un Tiempo límite de llamada de 5000 ms, el Cortocircuito con errores al 20 % y el Cortocircuito abierto durante 60 s. Los Servidores de aplicaciones están en una Zona de disponibilidad con una Malla de servicios Linkerd y Reintentos de la malla 3, y están conectados a una tarjeta API de Claude (4000 peticiones y 2 000 000 de tokens por minuto) y a una tarjeta API de OpenAI (1500 peticiones y 1 000 000 de tokens por minuto). Cuesta unos 141 766 $ al mes, y casi todo son tokens: 81 266 $ de la API de Claude, 60 335 $ de la API de OpenAI, 140 $ de los Servidores de aplicaciones y 25 $ del ALB. La factura de tokens usa los precios de lista del registro por millón de tokens, como si el pico durara todo el mes. Un día normal tiene un p95 de unos 2479 ms para la respuesta completa, un p95 del tiempo hasta el primer token de unos 0,9 s, y ningún error.

Cada ajuste hace un trabajo:

- **El tiempo límite de llamada** corta la espera. 5 s está por encima de casi todas las respuestas normales, así que en un día normal no corta nada. Un tiempo límite más corto corta respuestas largas y sanas, y cada respuesta cortada se paga dos veces: con 3000 ms el día normal cuesta unos 148 055 $.
- **El cortocircuito** deja de pagar el tiempo límite en cada llamada. Cuando al menos 20 llamadas a un proveedor en los últimos 10 s han fallado en un 20 % o más, el Servidor de aplicaciones hace fallar al instante sus llamadas a ese proveedor durante 60 s. Con el Cortocircuito abierto durante 10 s, el cortocircuito se cierra mientras el proveedor sigue colgado, las siguientes llamadas vuelven a esperar 5 s cada una, y el p95 del tiempo hasta el primer token supera el límite.
- **El reintento** convierte una llamada fallida en una respuesta. Un reintento va al siguiente proveedor por turno. Tras un cortocircuito abierto, siempre es el otro proveedor; tras un tiempo límite puede volver a ser el mismo proveedor colgado, y por eso la referencia permite 3 reintentos. Con 2, alrededor del 0,15 % de los mensajes se quedan sin intentos durante el cuelgue: sigue aprobando, pero con poco margen.

"Un proveedor de modelos cae durante 60 s" mata la primera tarjeta de modelo de chat (por orden de id de tarjeta) a los 20 segundos, durante 60 segundos, en una ejecución de 100 segundos. Las llamadas a ella fallan enseguida, la malla las reintenta en el otro proveedor, y a los 4 s más o menos las comprobaciones de salud sacan la tarjeta muerta del turno. No hay errores, y el p95 se queda en unos 2430 ms.

"Un proveedor de modelos se cuelga durante 60 s" añade 60 s a cada llamada en esa tarjeta durante el mismo minuto. En los primeros segundos, los mensajes enviados allí esperan al tiempo límite, y luego el reintento los manda al otro proveedor. Tras unos segundos de tiempos límite el cortocircuito se abre, y desde entonces las llamadas que caen en el proveedor colgado fallan al instante y se reintentan en el otro sin esperar. No hay errores, y el p95 de la respuesta completa es de unos 2737 ms. El p95 del tiempo hasta el primer token está entre 5,5 s y 6 s, por debajo del límite de 8 s de esta prueba: cuenta los mensajes que esperaron primero a un tiempo límite, y los primeros tokens de las llamadas que se quedaron colgadas.

Para el reto de coste, la búsqueda de pruebas no encontró ningún diseño por debajo del 80 % del coste de la referencia, así que este problema no tiene reto. La factura son tokens, y todos los diseños pagan los mismos tokens.

## Por qué falla el primer intento

Un primer intento habitual es el mismo borde y los mismos Servidores de aplicaciones con solo la tarjeta API de Claude, y sin tiempo límite de llamada. Cuesta unos 159 120 $, más que la referencia, porque cada token se paga al precio de la API de Claude, mientras que la referencia paga la mitad al precio de lista más bajo de la API de OpenAI. Supera "Un día normal" con un p95 de unos 2630 ms.

Cuando el proveedor cae, no hay otro sitio adonde ir: alrededor del 61,7 % de los mensajes fallan, todos en ese minuto.

Cuando el proveedor se cuelga, es peor. Los mensajes esperan hasta que el Cliente web se rinde a los 30 s, así que los primeros errores llegan 30 s después de que empiece el cuelgue, y los mensajes que salen adelante cuando termina llevaban mucho tiempo atascados: alrededor de un 35,0 % de errores y un p95 de unos 24 168 ms.

Añadir el segundo proveedor no basta por sí solo. Con las dos tarjetas y sin tiempo límite, cortocircuito ni reintento, "cae" hace fallar alrededor del 1,8 % de los mensajes (los 4 s antes de que lo detecten las comprobaciones de salud), y "se cuelga" alrededor del 20,0 %: la mitad de las llamadas van a un proveedor que parece sano y nunca responde. Cada pieza que falta también se nota:

- Un tiempo límite y reintentos sin el cortocircuito: alrededor de un 2,3 % de errores y un p95 de unos 11 616 ms durante el cuelgue. Cada llamada al proveedor colgado espera 5 s antes de pasar al otro, y el simulador factura las llamadas que se quedan colgadas: el coste de esa prueba sube a unos 203 895 $.
- Un tiempo límite y el cortocircuito sin reintentos: alrededor de un 34,7 % de errores. El cortocircuito falla rápido, pero un fallo rápido sigue siendo un fallo.

La idea es esta: un proveedor de respaldo solo sirve si el tráfico llega de verdad a él en segundos. Un proveedor colgado parece sano, así que necesitas un tiempo límite para notarlo, un cortocircuito para dejar de pagar ese tiempo límite en cada llamada, y un reintento para mandar el mensaje al respaldo. Y el respaldo tiene que aguantar toda la carga dentro de sus propios límites.

## Lo que el simulador deja fuera

La calidad de las respuestas no se simula: los modelos de los dos proveedores dan respuestas distintas, y nada aquí mide eso, ni el trabajo de que un mismo prompt funcione bien en los dos. El tiempo límite de llamada cubre la respuesta completa, mientras que los clientes reales con streaming suelen poner además un tiempo límite propio al primer token. El failover real suele preferir el proveedor principal y usar el respaldo solo cuando este falla; aquí un Servidor de aplicaciones envía sus llamadas a los dos por turnos, así que el respaldo sirve la mitad del tráfico todos los días. El cortocircuito no tiene sondas semiabiertas. El simulador factura una llamada que empieza aunque nadie espere su respuesta; lo que un proveedor real cobra por una llamada abandonada no se modela. Las páginas de estado de los proveedores y las cabeceras Retry-After no se simulan. Los precios son los precios de lista del registro por millón de tokens.

## Para probar

- Vuelve a poner el Límite de tokens por minuto de la API de OpenAI en su valor por defecto de 400 000. Un día normal sigue aprobando, porque cada proveedor lleva solo la mitad de la carga. Luego ejecuta la caída y mira qué pasa cuando la API de OpenAI tiene que llevarla entera.
- Pon los Reintentos de la malla en 2, luego en 0, y ejecuta el cuelgue. Cuenta los mensajes que se quedan sin intentos.
- Pon el Cortocircuito abierto durante en 10 s y mira el tiempo hasta el primer token durante el cuelgue.
