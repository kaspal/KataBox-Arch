# Solución comentada

## Adónde va la carga

Con 1000 transferencias/s, el libro mayor recibe 1000 confirmaciones por segundo. PostgreSQL dedica unos 5 ms a cada una, así que unas 5 de sus 100 conexiones están ocupadas: un solo primario va sobrado. Cada transferencia pasa también por el stream y acaba en una escritura en DynamoDB, que gasta tres veces la capacidad de una lectura y se factura.

Un stream mantiene un solo mensaje en curso por partición. Un proyector dedica unos 2 ms a cada cambio, más una escritura en DynamoDB de unos 3 ms, así que 1000 cambios por segundo mantienen ocupadas unas 5 particiones, y el doble durante una ráfaga.

La velocidad no es lo difícil. Lo difícil es que una transferencia ya respondida no puede perderse nunca.

## El diseño de referencia

Un Load Balancer (AWS ALB) envía las transferencias a la Ledger API, 2 instancias Go. Cada transferencia se confirma en un libro mayor PostgreSQL, y un segundo PostgreSQL está conectado a él con un enlace Replicate. Los dos están conectados a un stream Kafka de 32 particiones por enlaces de captura de cambios. 6 Balance projectors Celery, con un tiempo de servicio de 2 ms, consumen el stream y actualizan los saldos en DynamoDB. Cuesta unos 3249 $ al mes: DynamoDB unos 1426 $, la mayor parte por el precio de 2600 millones de escrituras al mes, y Kafka unos 538 $. El p95 hasta completar es de unos 50 ms.

«Una tormenta de reintentos» duplica el tráfico durante 15 segundos a partir del segundo 20, y admite un p95 de 15 s. Los cambios de más esperan su turno en el stream y en los proyectores, y el p95 sube a unos 230 ms.

«Falla el primario del libro mayor en plena transferencia» tumba el primario de PostgreSQL en el segundo 20, para siempre, en una ejecución de 120 segundos. Admite un 30 % de errores y un 70 % de disponibilidad, pero ninguna transferencia perdida. Las transferencias fallan hasta que la réplica pasa a primario, tras el tiempo de conmutación por defecto de PostgreSQL, 30 segundos: 30 de 120 segundos, cerca del 25 % de la ejecución. Esos clientes ven un error y pueden volver a intentarlo. Una escritura fallida nunca se confirma, así que no publica nada, y la réplica promovida sigue publicando por su propio enlace de captura de cambios. No se pierde nada.

## Por qué falla el primer intento

Un primer intento habitual es el mismo diseño con un solo libro mayor PostgreSQL y sin réplica, por unos 2899 $ al mes. Supera «Un día normal» y «Una tormenta de reintentos» con las mismas cifras que la referencia.

Cuando cae el primario no hay nada que promover, así que todas las transferencias fallan desde el segundo 20 hasta el final de la ejecución: cerca de un 83 % de errores. No se pierde nada confirmado, pero la app de pagos está caída hasta que alguien reconstruya la base de datos.

La lección: un error que ve el cliente se puede reintentar, pero una transferencia confirmada y luego perdida no. El libro mayor tiene que estar replicado, y los saldos son una proyección que siempre puedes reconstruir desde su registro de cambios.

## Lo que el simulador deja fuera

No se simulan la entrega exactamente una vez ni la idempotencia: una transferencia reintentada es simplemente una nueva, sin una clave que detecte el duplicado. Tampoco se modela la contabilidad por partida doble. Aquí la replicación es asíncrona, con la réplica 50 ms por detrás, y aun así el simulador no pierde ninguna transferencia confirmada en la conmutación. La replicación asíncrona real sí puede perderlas: las últimas transferencias confirmadas antes de la caída pueden no llegar nunca a la réplica. Por eso los libros mayores reales replican de forma síncrona, o a un quórum, y lo pagan con confirmaciones más lentas.

## Para probar

- Borra el enlace de captura de cambios de la réplica al stream y ejecuta «Falla el primario del libro mayor en plena transferencia». Los cambios confirmados después de la promoción no tienen adónde ir.
- Vuelve a dejar el stream en 6 particiones y ejecuta «Una tormenta de reintentos».
- Baja el tiempo de conmutación del libro mayor a 10 s y compara la tasa de errores.
