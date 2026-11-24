# Solución comentada

## Adónde va la carga

Con 3000 peticiones/s, unas 2970 son visitas y unas 30 son lecturas del panel. Cada visita va de la Collector al stream y de ahí a un Counter, que dedica 2 ms al incremento y escribe en PostgreSQL. Con los saltos y la escritura, contar una visita lleva unos 8 a 10 ms.

Cada partición entrega un mensaje cada vez, así que el número de particiones limita cuántas visitas se cuentan a la vez, por muchos Counters que añadas. A unos 9 ms por visita, una partición cuenta unas 110 visitas por segundo: 32 particiones cuentan unas 3500 a 4000 por segundo y 48 cuentan unas 5000 a 6000. Los Counters necesitan al menos tantos huecos como particiones; si no, algunas particiones se quedan paradas.

## El diseño de referencia

La Collector, con 2 instancias Go detrás de un Load Balancer, publica cada visita en un stream Kafka con 48 particiones. Lo consumen 8 instancias Celery de Counters; con la concurrencia por defecto de Celery, 8, son 64 huecos, suficientes para mantener ocupadas las 48 particiones. Los Counters escriben en PostgreSQL. Para el panel, la Collector lee primero una Cache Redis con un TTL de 10 segundos y, si no encuentra el dato, la llena desde PostgreSQL. Cuesta unos 1162 $ al mes, y en un día normal las visitas se cuentan con un p95 de unos 32 ms.

«La portada se hace viral» duplica el tráfico durante 15 segundos, a partir del segundo 20, y admite un p95 de 10 segundos. Llegan unas 5940 visitas por segundo, en el límite de lo que pueden contar 48 particiones, así que se forma un atasco que luego se vacía: el p95 queda en unos 5,5 s. «Los consumidores se ralentizan» suma 10 ms a cada mensaje en todas las tarjetas Worker durante 20 segundos, a partir del segundo 15, con el mismo límite de 10 s. A unos 19 ms por visita, 48 particiones solo cuentan unas 2500 por segundo, algo menos de lo que llega. El pequeño atasco se vacía cuando los Workers se recuperan: el p95 queda en unos 4,4 s.

La caché aparta las lecturas del panel de una base de datos muy ocupada. Sus cifras pueden ir hasta 10 segundos por detrás, y el juez marca como desactualizadas cerca del 25 % de las lecturas del panel: para un panel de estadísticas no pasa nada, así que aquí no se evalúa.

## Por qué falla el primer intento

Un primer intento habitual usa 32 particiones y 4 instancias de Counters, es decir, 32 huecos, y ninguna caché: el panel lee PostgreSQL directamente. Supera «Un día normal», pero con un p95 de unos 2,4 s, ya cerca del límite de 3 s. 2970 visitas por segundo son casi todo lo que pueden contar 32 particiones, así que cualquier pico hace cola.

La prueba viral supera con mucho ese techo: el atasco crece durante los 15 segundos y el p95 llega a unos 18,3 s. Cuando los Workers se ralentizan, 32 particiones solo cuentan unas 1700 visitas por segundo y el p95 llega a unos 12,1 s. Añadir instancias de Counters no arreglaría ninguna de las dos pruebas: el límite son las particiones.

## Lo que el simulador deja fuera

Los contadores reales suman muchas visitas en memoria y escriben el total de cada página una vez por segundo, lo que reduce muchísimo las escrituras en la base de datos. El simulador cuenta cada visita por separado, así que este problema mide las particiones y la capacidad de los consumidores.

Tampoco se simula el conteo exactamente una vez. En la vida real, una visita reintentada podría contarse dos veces.

## Para probar

- Deja 32 particiones y sube los Counters a 16 instancias. El p95 viral apenas cambia.
- Pon 48 particiones pero deja 4 instancias de Counters (32 huecos). Observa cuántas particiones se quedan paradas.
- Baja el TTL de la caché a 2 segundos y compara la tasa de lecturas desactualizadas.
