# Solución comentada

## Adónde va la carga

Un almacén de datos no ejecuta las consultas de una en una: cada clúster tiene 8 huecos, y cada consulta ocupa un hueco hasta que termina. Así que la pregunta es cuántos huecos están ocupados a la vez: consultas por segundo × segundos por consulta.

Una consulta ad hoc tarda los 300 ms de sobrecarga más 0,1 GB a 1 GB/s: unos 0,4 s. Solo el 5 % repite una consulta anterior, así que la Caché de resultados responde a pocas y las demás escanean. En una hora normal, 12 consultas por segundo mantienen ocupados unos 12 × 0,38 ≈ 4,6 huecos. Un clúster tiene 8, así que aguanta.

A las 9 de la mañana llegan cuatro veces más: unos 18 huecos ocupados. Un clúster tiene 8; dos tienen 16, y siguen sin llegar. Cuando las llegadas superan a los huecos, la cola crece cada segundo, y cada analista espera a todos los que tiene delante. La caché no ayuda aquí, porque todas las consultas son distintas. Solo ayudan más huecos.

Una consulta de exploración escanea 60 GB y ocupa su hueco unos 60 s. Las pocas de un día normal no importan. Cuando el equipo de ciencia de datos se pone a explorar, llegan unas 0,48 por segundo, y tras el primer minuto unos 0,48 × 60 ≈ 29 huecos están ocupados solo por notebooks. En un almacén que comparten los analistas, esos huecos ya no son para ellos.

## El diseño de referencia

El Cliente tiene un Tiempo límite de 120 000 ms. El Servidor de aplicaciones (2 instancias Go) ejecuta las consultas de los analistas en un Almacén de datos Snowflake con 3 instancias: 24 huecos. El Microservicio (2 instancias) ejecuta las consultas de los notebooks en un segundo Almacén de datos Snowflake con 5 instancias: 40 huecos. Los dos dejan la Concurrencia por instancia en 8 y la Suspensión automática tras en 60 s.

En una hora normal cuesta unos 4660 $ al mes: 140 $ los Servidores de aplicaciones, 140 $ el Microservicio, 4380 $ el almacén de los analistas y 0 $ el de los notebooks. En esa hora no llegó ninguna consulta de exploración, así que el segundo almacén siguió suspendido y no facturó nada. El p95 es de unos 417 ms, sin errores.

"Las 9 de la mañana: cuatro veces las consultas durante 40 s" lleva unos 18 huecos ocupados al almacén de los analistas: 24 huecos los ejecutan a un 76 % más o menos. El p95 es de unos 443 ms, sin errores. Con 2 instancias, la misma avalancha llega a un p95 de unos 7,1 s.

"El equipo de ciencia de datos se pone a explorar todo el histórico" manda unos 29 huecos de exploración al almacén de los notebooks, que los ejecuta en 40 huecos. El almacén de los analistas no las ve: el p95 sigue en unos 417 ms, sin errores. Las consultas de exploración tardan unos 60 s cada una, pero son menos del 4 % de las consultas, así que quedan por encima del p95.

El presupuesto se mide en una hora normal. Mientras el almacén de los notebooks trabaja, factura como cualquier otro: en las pruebas ocultas el mismo diseño marca un ritmo mensual de unos 10 300 $ y 11 900 $. La suspensión automática es lo que hace que ese coste sea solo el de las horas de trabajo.

¿Por qué no un único almacén compartido con más clústeres? Solo funciona con huecos suficientes para las dos cargas a la vez. Con 3 instancias, la prueba de exploración llega a un p95 de unos 44,7 s; con 4 instancias, a unos 8,6 s. Con 5 pasa, pero cuesta unos 7580 $, por encima del presupuesto: los analistas mantienen ocupado todo el día un almacén compartido, así que sus 5 clústeres facturan todo el día, mientras que un almacén aparte para los notebooks solo factura mientras exploran.

Para el reto de coste, la búsqueda no encontró nada por debajo del 80 % del coste de la referencia (lo mejor fue un 97 %, con una instancia menos del Servidor de aplicaciones), así que este problema no tiene reto.

## Por qué falla el primer intento

Un primer intento habitual conecta el Servidor de aplicaciones y el Microservicio a un único Almacén de datos Snowflake con 1 instancia. Cuesta unos 1740 $ y supera "Una hora normal de trabajo" con un p95 de unos 479 ms: 4,6 huecos ocupados caben en 8.

A las 9 de la mañana llegan consultas para unos 18 huecos a 8 huecos. La cola crece desde los primeros segundos de la avalancha: el p95 de cada segundo sube de unos 2,3 s a unos 49 s, y la cola no se vacía antes de que termine la ejecución. La prueba acaba con un p95 de unos 65 s y un 2,39 % de las consultas rechazadas cuando la cola está llena.

Cuando los notebooks se ponen a explorar, las consultas de exploración se quedan con todos los huecos y los mantienen un minuto. Las consultas de los analistas esperan detrás, la cola se llena y el almacén rechaza lo que no cabe: falla más o menos el 69,9 % de las consultas, con un p95 de unos 65 s.

La idea es esta: la capacidad de un almacén son huecos, clústeres × concurrencia, y se dimensiona para la avalancha, no para la media. Y una carga que ocupa huecos durante minutos no debe ir con otra que necesita respuestas en segundos: dale su propio almacén, y deja que la suspensión automática lo haga barato.

## Lo que el simulador deja fuera

Los almacenes multiclúster reales añaden y quitan clústeres según crece y baja la cola (entre un mínimo y un máximo); aquí los clústeres son fijos, y la tarjeta los factura todos mientras corre cualquier consulta. Reanudar un almacén suspendido tarda unos segundos en la vida real; aquí es instantáneo. Subir la Concurrencia por instancia aquí no hace más lentas las consultas, mientras que las consultas reales de un clúster comparten su CPU y su memoria; y un tamaño de almacén mayor (una Velocidad de escaneo por consulta más alta) aquí no cuesta más. Por eso el enunciado fija el tamaño. No se simulan los tiempos límite de la cola, las reglas de enrutado por usuario ni la corrección de los propios datos: separar las cargas se dibuja como cableado. El precio de Snowflake es la cifra didáctica del registro (2 $ por crédito), y el tiempo está acortado: la avalancha dura 40 s y la exploración 2 minutos.

## Para probar

- Pon el almacén de los analistas en 2 instancias y ejecuta la avalancha de las 9. Mira cómo crece la cola segundo a segundo.
- Conecta el Microservicio al almacén de los analistas, con 4 instancias, y ejecuta la prueba de exploración.
- Pon la Concurrencia por instancia del primer intento en 32 y ejecuta la avalancha. Después vuelve a leer "Lo que el simulador deja fuera": ¿por qué no funcionaría eso en un almacén real?
