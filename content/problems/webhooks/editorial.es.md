# Solución comentada

## Adónde va la carga

Con 2000 eventos/s, unos 1400 van a los endpoints de los clientes en general y unos 600 van a Acme. El trabajo es esperar: un Sender ocupa un hueco en cada llamada. Solo con las medianas, 1400 × 0,2 s + 600 × 0,1 s mantienen ocupados unos 340 huecos, y la cola lenta de los endpoints generales añade muchos más.

Las pruebas ocultas cambian las cuentas. Cuando Acme está caído, cada uno de sus 600 eventos por segundo ocupa un hueco durante el tiempo de conexión agotado de 1 s solo para fallar, y cada reintento vuelve a ocupar uno. Cuando los endpoints generales se vuelven 1 s más lentos, sus 1400 llamadas por segundo tardan unos 1,2 s cada una: unos 1700 huecos solo para ellas, sin contar Acme ni la cola lenta.

## El diseño de referencia

La Events API funciona con 2 instancias Go y publica en una Queue SQS con max_retries en 8. Desde esa cola sale una Dead-letter Queue. La consumen 28 Senders Celery, cada uno con concurrencia 64: 1792 huecos en total. Tener muchos huecos por instancia tiene sentido aquí, porque enviar es esperar a la red, no usar la CPU. Cuesta unos 4243 $ al mes, con un p95 cercano a 1 s en un día normal.

«El endpoint de Acme cae durante un minuto» tumba Acme en el segundo 20 durante 60 segundos, en una ejecución de 150 segundos. Admite que la mitad de las peticiones den error y un p95 de 120 segundos, pero ningún mensaje perdido. Los reintentos esperan 1, 2, 4, 8, 16, 32, 32 y 32 segundos, unos dos minutos en total, lo que dura más que la caída, así que todos los eventos llegan a Acme cuando vuelve, con un p95 de unos 41 segundos. Lo que aún fallara acabaría en la Dead-letter Queue. «Los endpoints de los clientes se vuelven lentos» añade 1 s a cada llamada a los endpoints generales a partir del segundo 15, durante 30 segundos, con un límite de p95 de 10 segundos. Los 1792 huecos lo soportan con un p95 de unos 5,9 segundos.

## Por qué falla el primer intento

Un primer intento habitual es la misma cola con sus 3 reintentos por defecto y sin Dead-letter Queue, y 22 Senders de 64: 1408 huecos, por unos 3823 $ al mes. Supera «Un día normal» con el mismo p95 cercano a 1 s.

Durante la caída de Acme, cada evento falla tras 1 s y se reintenta tras 1, 2 y 4 segundos. Esas esperas suman solo 7 segundos, así que los reintentos se agotan con Acme todavía caído y se pierden unos 5198 mensajes. Cuando los endpoints se vuelven lentos, los 1408 huecos no bastan para las llamadas lentas, se forma un atasco en la cola y el p95 llega a unos 10,7 segundos, por encima del límite.

## Lo que el simulador deja fuera

El siguiente paso habitual es aislar a cada cliente: una cola y un grupo de Senders propios para cada cliente grande, de modo que la caída de uno no pueda quitarles los huecos a todos. El simulador no dirige las llamadas a un proveedor concreto a través de un grupo concreto, porque los proveedores externos no son tarjetas, así que no puede mostrar ese aislamiento; este problema mide en cambio los reintentos, los mensajes fallidos y la capacidad de envío. Tampoco se simulan las firmas, las claves de idempotencia ni el orden por cliente.

## Para probar

- Mantén 8 reintentos pero quita la Dead-letter Queue. Después prueba 5 reintentos (31 segundos de esperas) con ella y mira adónde van los eventos fallidos.
- Vuelve a dejar los Senders en 22 instancias y ejecuta otra vez «Los endpoints de los clientes se vuelven lentos». Después prueba una concurrencia de 8, la de Celery por defecto, y cuenta cuántas instancias necesitarías.
