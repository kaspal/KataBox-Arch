# Solución comentada

## Adónde va la carga

Llegan 20 000 peticiones por segundo, y el proveedor acepta como mucho 200 llamadas por segundo. Sin caché, se rechazaría el 99 % de las peticiones. Así que el trabajo de verdad lo hace la caché, y la pregunta es cada cuánto tiene que preguntar al proveedor.

Hay 40 000 ciudades. Con un TTL de 600 s, cada ciudad caduca más o menos una vez cada 10 minutos, así que una caché caliente pide al proveedor unas 40 000 / 600 ≈ 67 previsiones por segundo, muy por debajo del límite de 200. Una caché fría es otra historia: volver a llenar las 40 000 ciudades a 200 llamadas por segundo lleva 200 s, y mientras tanto la mayoría de las peticiones fallan en la caché.

Las pruebas ocultas se llevan cada una de esas dos mitades. Cuando el proveedor cae durante 40 segundos, siguen caducando unas 67 ciudades por segundo, y nadie puede refrescarlas. Cuando se reinicia el nodo de caché, lo que desaparece es la propia copia.

## El diseño de referencia

Un AWS ALB reparte el tráfico entre 8 Go App Servers con un tiempo límite de llamada de 200 ms. Leen de una Redis Cache de 2 GB con un TTL de 600 s, «Servir caducado ante error durante» en 3600 s, la protección de estampida activada y un tiempo de failover de 5 s. Está conectada con Replicar a una segunda Redis Cache con los mismos ajustes. En un fallo de caché, el App Server llama al proveedor y la respuesta llena la caché. Cuesta unos 634 $ al mes: 560 $ los App Servers, 25 $ el ALB y 24 $ cada Redis. Un día normal tiene un p95 de unos 19,0 ms, ningún error y un 0,58 % de lecturas obsoletas.

«El proveedor del tiempo cae durante 40 s» detiene el proveedor a partir del segundo 20, durante 40 segundos, en una ejecución de 90 segundos. Cuando una ciudad ha caducado y la llamada al proveedor falla, la caché sirve la copia caducada en vez de un error, y las peticiones que esperan al fallo de esa misma ciudad reciben también esa copia. El p95 queda en unos 22,3 ms, sin errores y con un 2,98 % de lecturas obsoletas. El precio de servir caducado ante error es que la petición primero espera a que falle la llamada. A medida que caducan más ciudades durante la caída, más peticiones esperan a la vez. Por eso los App Servers tienen margen y un tiempo límite de llamada corto: con 6 instancias y un tiempo límite de 300 ms, los últimos 10 segundos de la caída llegaron hasta un 24 % de errores, porque todos los huecos de los App Servers estaban ocupados.

«Un nodo de caché se reinicia» mata la primera Redis Cache en el segundo 20, durante 5 segundos. Esta prueba admite un 3 % de errores y un 97 % de disponibilidad. El diseño falla en un 1,65 % de las peticiones, todas en los 2 segundos más o menos que tardan las comprobaciones de salud en notar el nodo caído; después la réplica toma el relevo. Cuando el nodo vuelve, se sincroniza con la copia de la réplica en vez de arrancar vacío, así que el proveedor nunca recibe una avalancha. El p95 se mantiene en unos 19,0 ms.

Para el reto de coste, un diseño de prueba se queda en el 72 % del coste de la referencia, por debajo del objetivo del 75 %. Mantiene la misma forma y los mismos ajustes de caché, con menos App Servers y cachés mucho más pequeñas: 40 000 previsiones ocupan muy poca memoria.

## Por qué falla el primer intento

Un primer intento habitual es la misma entrada con 6 Go App Servers sin tiempo límite de llamada, y una sola Redis Cache de 2 GB con un TTL de 600 s: sin servir caducado ante error, sin protección de estampida y sin réplica. Cuesta unos 469 $ y supera «Un día normal» con un p95 de unos 18,6 ms.

Durante la caída, cada ciudad que caduca no tiene nada que responder: la llamada al proveedor falla y la petición falla con ella. Falla un 26,7 % de las peticiones, y el p95 sube a unos 519 ms porque las peticiones esperan a llamadas que nunca salen bien.

El reinicio es peor: un 59,3 % de errores. La caché vuelve vacía, y 20 000 peticiones por segundo fallan en ella a la vez. El proveedor rechaza todo lo que pase de 200 llamadas por segundo, y volver a llenar 40 000 ciudades a ese ritmo lleva minutos.

La idea es esta: delante de un proveedor lento y limitado, la caché es tu disponibilidad, no solo tu velocidad. Una copia caducada es mejor que un error, así que guárdala para cuando falle el proveedor. Y no dejes nunca que la caché vuelva vacía: una réplica que pueda tomar el relevo y sincronizarla mantiene toda la carga lejos del proveedor.

## Lo que el simulador deja fuera

No se simula la persistencia de Redis (las instantáneas RDB o el registro AOF), así que una Redis sola siempre vuelve vacía. El stale-while-revalidate real responde con la copia caducada al momento y la refresca en segundo plano; aquí la petición espera primero a la llamada fallida. Una previsión real cambia según el calendario del proveedor, no cuando se acaba tu TTL. No se cobra el precio por llamada del proveedor, y tampoco se simulan las cabeceras Retry-After.

## Para probar

- Desactiva la protección de estampida y vuelve a ejecutar la caída. Mira cuántas llamadas más llegan al proveedor y cómo cambian los errores.
- Pon los App Servers en 6 instancias y el tiempo límite de llamada en 300 ms, y ejecuta la caída. Fíjate en sus últimos 10 segundos.
- Quita la réplica y ejecuta el reinicio. Compara sus errores con el límite de 200 llamadas por segundo del proveedor.
