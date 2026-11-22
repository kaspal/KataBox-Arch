# Solución comentada

## Adónde va la carga

Con 1 500 notificaciones/s, unas 900 son push, 450 son correos y 150 son SMS. El coste real es el tiempo de los Workers, que se va sobre todo en esperar a los proveedores. En un día normal hay unos 150 huecos de envío ocupados de media, más en las colas lentas.

Un proveedor caído cambia las cuentas. Cada llamada a él falla solo tras un tiempo de conexión agotado de 1 s, así que 450 correos por segundo ocupan unos 450 huecos solo para fallar, y cada reintento vuelve a ocupar uno. Si los Workers se quedan sin huecos, el push y los SMS hacen cola detrás de correos que no pueden ir a ninguna parte.

## El diseño de referencia

La Notify API funciona con 2 instancias Go y publica en una Queue SQS con max_retries en 10. Desde esa cola sale una Dead-letter Queue. La consumen 32 Senders Celery, cada uno con concurrencia 32: 1 024 huecos en total. Cuesta unos 4 001 $ al mes.

«Se cae el proveedor de correo» detiene el correo durante 40 segundos, a partir del segundo 15. Admite un p95 de 60 segundos y ningún mensaje perdido. Los envíos fallidos que se reintentan no son errores (la notificación sigue en camino); Resultados los cuenta aparte como intentos reintentados. Los reintentos llegan tras 1, 2, 4, 8, 16 y 32 segundos, lo que dura más que la caída, así que los correos salen cuando el proveedor vuelve. Lo que aún fallara acabaría en la Dead-letter Queue. «Un envío masivo de marketing» manda 5 veces el tráfico durante 10 segundos y admite un p95 de 15 segundos; este diseño lo resuelve con un p95 de cerca de 1,2 segundos.

## Por qué falla el primer intento

Un primer intento habitual es el mismo diseño, pero la cola mantiene sus 3 reintentos por defecto y no tiene Dead-letter Queue. Supera «Un día normal» y el envío masivo. Durante la caída, cada correo falla tras 1 s y se reintenta tras 1, 2 y 4 segundos. Los tres reintentos se agotan en unos 11 segundos, con el proveedor todavía caído. Se pierden unos 4 575 mensajes y la prueba falla.

La lección: una caída más larga que tu calendario de reintentos pierde mensajes, salvo que los reintentos duren lo suficiente o que los fallos acaben en una cola de mensajes fallidos para reenviarlos después. La capacidad de envío también importa.

## Para probar

- Mantén 10 reintentos pero quita la Dead-letter Queue. Después prueba 5 reintentos con ella y mira adónde van los fallos.
- Baja los Senders a 8 instancias y observa la latencia del push y los SMS durante la caída del correo.
