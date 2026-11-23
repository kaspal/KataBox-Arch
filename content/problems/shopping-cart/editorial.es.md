# Solución comentada

## Adónde va la carga

Con 6000 rps, 3600 peticiones por segundo son vistas del carrito, 1800 son cambios en el carrito y 600 son pagos. Las vistas pueden salir de una caché; las 2400 escrituras por segundo tienen que llegar al primario de PostgreSQL.

La multitud cae sobre la capa de aplicación. Cada petición ocupa un hueco de la aplicación durante al menos sus 40 ms de trabajo, más lo que espera a la caché o a la base de datos. Cuando empiezan las rebajas, el tráfico sube hasta 5 veces el pico, 30 000 rps, y el grupo de autoescalado añade instancias para seguir el ritmo. Una instancia Spring Boot con concurrencia 200 tiene un pool de 200 hilos, y cada hilo que habla con PostgreSQL directamente mantiene una conexión abierta. Así que cada instancia que añade el grupo trae 200 conexiones más, y PostgreSQL admite como mucho 1700. 8 instancias ocupan 1600 conexiones; una 9.ª lo lleva a 1800, por encima del límite.

Así que la pregunta no es solo cuántas instancias de la aplicación necesitas, sino cuántas conexiones a la base traen consigo.

## El diseño de referencia

Un AWS ALB envía a los compradores a Servidores de aplicaciones Spring Boot con concurrencia 200 y 40 ms de trabajo por petición, dentro de un AWS Auto Scaling Group con un mínimo de 6 instancias, un máximo de 24, una utilización objetivo del 60 % y un retardo de escalado de 20 s. Las vistas del carrito pasan en modo cache-aside por una caché Redis con 8 GB de memoria y un TTL de 600 s. Todas las llamadas a la base pasan por un Pool de conexiones PgBouncer en modo transacción con 800 conexiones a la base de datos, hasta PostgreSQL con 2 shards.

Cuesta unos 1319 $ al mes en un día normal: 700 $ de PostgreSQL, 420 $ de los Servidores de aplicaciones, 144 $ de Redis, 30 $ de PgBouncer y 25 $ del ALB. Durante las rebajas sube a unos 1489 $ a medida que el grupo escala, todavía muy por debajo del presupuesto de 2200 $. En «Una tarde normal» el p95 es de unos 117 ms, sin errores, y los Servidores de aplicaciones están ocupados en torno al 40 %.

El pooler es lo que deja crecer a la capa de aplicación. Tenga el grupo las instancias que tenga, PostgreSQL ve como mucho las 800 conexiones de PgBouncer. En modo transacción, una conexión se presta a una petición solo durante su transacción y luego vuelve al pool, así que 800 conexiones atienden a muchos más hilos de la aplicación, la mayoría de los cuales están haciendo otra cosa en cada momento.

«Empiezan las rebajas» sube el tráfico de 1 a 5 veces el pico a lo largo de 180 s. El grupo va añadiendo instancias, el pool se queda en 800, y el p95 se mantiene en unos 117 ms, sin errores.

«Una oferta relámpago a mediodía» sube el tráfico de golpe a 2,5 veces el pico en el segundo 20 y lo mantiene ahí el resto de la ejecución de 120 s. El suelo de 6 instancias tiene margen para aguantar el salto mientras el grupo crece por detrás: el p95 es de unos 117 ms, sin errores.

Este problema no tiene reto de coste. El diseño más barato que encontramos que supera todas las pruebas costaba en torno al 90 % de la referencia: la base de datos es más de la mitad de la factura, y cada escritura la necesita.

## Por qué falla el primer intento

Un primer intento habitual es el mismo diseño sin el pooler: los Servidores de aplicaciones se conectan a PostgreSQL directamente, y el mínimo del grupo es de 4 instancias. Cuesta unos 1149 $ al mes. Supera «Una tarde normal»: 4 × 200 = 800 conexiones, muy por debajo de 1700.

También supera «Una oferta relámpago a mediodía», con un p95 de unos 138 ms: a 2,5 veces el pico el grupo solo necesita unas 7 instancias, 1400 conexiones.

«Empiezan las rebajas» es donde se rompe. Hacia el segundo 120, el grupo pasa de 8 instancias, y 9 × 200 = 1800 conexiones son más que las 1700 de PostgreSQL. La base rechaza las conexiones nuevas, y las peticiones que las necesitaban fallan: un 3 % por segundo al principio, luego un 9 %. En toda la ejecución falla el 4,0 % de las peticiones, por encima del límite del 1 %. Escalar, lo que debía sostener las rebajas, es lo que las rompe.

La idea es esta: autoescalar la capa de aplicación multiplica las conexiones a la base de datos. Un pooler como PgBouncer o RDS Proxy mantiene un pool fijo hacia la base, haya las instancias de la aplicación que haya, y el pooling por transacción comparte cada conexión entre muchas peticiones.

## Lo que el simulador deja fuera

No se simula la persistencia de Redis, ni la pérdida de carritos cuando Redis falla; aquí cada cambio del carrito se escribe en PostgreSQL. Las sesiones y la comprobación de inventario al pagar no se modelan. El propio pooler nunca falla, las sentencias preparadas en modo transacción no dan problemas y abrir una conexión no lleva tiempo.

## Para probar

- Quita la tarjeta de PgBouncer, conecta los Servidores de aplicaciones directamente a PostgreSQL y ejecuta las rebajas. Fíjate en la tasa de errores cuando el grupo pasa de 8 instancias.
- Baja las Conexiones a la base de datos del pooler a 200 y vuelve a ejecutar las rebajas. ¿Se convierte el pool en una cola delante de PostgreSQL?
- Sin pooler, pon las Instancias máximas del grupo en 8. ¿Desaparecen los errores de conexión, y qué le pasa al p95 a cambio?
