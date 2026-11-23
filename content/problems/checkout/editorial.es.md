# Solución comentada

## Adónde va la carga

Con 4000 peticiones/s, unas 3200 son visitas a productos y unas 800 son pedidos. Con Zipf 1,1 sobre 50 000 productos, una caché responde la mayoría de las visitas.

El tiempo se va en los pedidos. Guardar el pedido lleva milisegundos; el cobro tarda unos 300 ms de mediana y hasta 1,5 s en la cola lenta. Quien haga esa llamada ocupa un hueco durante toda ella. Solo con la mediana, 800 pedidos por segundo × 0,3 s mantienen ocupados unos 240 huecos. Cuando el proveedor añade 2,5 s, cada cobro tarda unos 2,8 s, y esos mismos 800 pedidos por segundo necesitan unos 2240 huecos.

## El diseño de referencia

Un Load Balancer (AWS ALB) envía a los clientes a 4 instancias Go de la Shop API, con la concurrencia por defecto de 64: 256 huecos. Las visitas a productos siguen el patrón cache-aside a través de una Cache Redis con un TTL de 60 s hacia PostgreSQL, una principal y una réplica. El pedido se escribe en la principal y luego se publica en una Queue SQS con max_retries en 8 y una Dead-letter Queue detrás. La consumen 12 Workers de pago Celery, cada uno con concurrencia 64: 768 huecos. La respuesta al cliente sale en cuanto el pedido queda guardado y en la cola, así que el p95 ronda los 32 ms. Cuesta unos 4418 $ al mes.

«El proveedor de pagos se ralentiza» añade 2,5 s a cada llamada de pago a partir del segundo 15, durante 30 segundos. Los 768 huecos de los Workers son menos que los 2240 que necesitan los cobros lentos, así que se acumula trabajo en la cola, que se vacía cuando el proveedor se recupera. Los App Servers nunca esperan al proveedor, así que el p95 se queda en unos 32 ms. «Tormenta de doble clic (pedidos ×2)» duplica el tráfico de pedidos durante toda la ejecución, hasta unos 1600 pedidos por segundo. Cada uno sigue siendo una escritura rápida y una publicación: el p95 ronda los 35 ms. La factura sube a casi 7000 $ con los mensajes extra.

## Por qué falla el primer intento

Un primer intento habitual es la misma tienda sin cola: 6 instancias Go, 384 huecos, y cada una llama al proveedor de pagos dentro de la petición. Cuesta unos 1181 $ al mes y supera «Un día normal» con un p95 de unos 500 ms, porque cada pedido espera a su cobro. Los App Servers ya están ocupados en torno al 89 %.

Las dos pruebas ocultas los desbordan. Cuando el proveedor se ralentiza, los cobros necesitan unos 2240 huecos y hay 384. Cuando los pedidos se duplican, 1600 cobros por segundo necesitan unos 480 huecos solo con la mediana, más que los 384 disponibles. En las dos pruebas se descarta cerca del 40 % de todas las peticiones, también las visitas a productos: esperan los mismos huecos de App Server, y todos están atascados en un pago.

La cola no es gratis. Solo SQS cuesta aquí unos 2537 $ al mes, más del doble de toda la factura del primer intento, y los Workers suman 840 $. Lo que compra es aislamiento: un proveedor lento llena una cola en vez de la tienda.

## Lo que el simulador deja fuera

En el simulador, un doble clic crea de verdad dos pedidos; la prueba mide la carga extra, no la deduplicación. Un checkout real envía una clave de idempotencia para cobrar una sola vez la repetición. Tampoco se simula que la reserva de stock sea correcta con pedidos simultáneos: nada comprueba que no se venda de más.

## Para probar

- Borra la cola de la referencia y ejecuta «El proveedor de pagos se ralentiza». Observa cómo fallan las visitas a productos junto con los pedidos.
- Baja los Workers de pago a 4 instancias y ejecuta la misma prueba. El p95 del cliente apenas se mueve; mira en cambio la profundidad de la cola.
