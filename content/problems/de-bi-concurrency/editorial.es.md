# Solución comentada

## Adónde va la carga

Un almacén de datos no ejecuta las consultas de una en una: cada clúster tiene 8 huecos, y cada consulta ocupa un hueco hasta que termina. Pero los huecos reparten la velocidad de escaneo del clúster: un clúster escanea 1 GB/s en total, corran en él las consultas que corran, y una sola consulta nunca escanea más deprisa. Así que la pregunta es cuántos GB llegan por segundo, y cuántos GB por segundo pueden escanear los clústeres.

Una consulta ad hoc tarda los 300 ms de sobrecarga más 0,1 GB a 1 GB/s: unos 0,4 s mientras al almacén le sobra velocidad de escaneo. Solo el 5 % repite una consulta anterior, así que la Caché de resultados responde a pocas y las demás escanean. En una hora normal, 12 consultas por segundo traen unos 12 × 0,095 ≈ 1,14 GB/s. Un clúster escanea 1 GB/s, así que ya se queda atrás; dos aguantan.

A las 9 de la mañana llegan cuatro veces más: unos 4,6 GB/s. Cuando los GB que llegan superan la velocidad de escaneo, la cola crece cada segundo, y cada analista espera a todos los que tiene delante. La caché no ayuda aquí, porque todas las consultas son distintas. Solo ayudan más clústeres.

Una consulta de exploración escanea 60 GB. En un almacén compartido con 5 clústeres, una sola pone 12 s de escaneo por delante de cada consulta de analista que llega detrás. Cuando el equipo de ciencia de datos se pone a explorar, llegan unas 0,48 por segundo: 0,48 × 60 ≈ 29 GB/s, mucho más que los 1,1 GB/s de los analistas. En un almacén que comparten los analistas, esa velocidad de escaneo ya no es para ellos.

## El diseño de referencia

El Cliente tiene un Tiempo límite de 120 000 ms. El Servidor de aplicaciones (2 instancias Go) ejecuta las consultas de los analistas en un Almacén de datos Snowflake con 5 instancias: 5 GB/s. El Microservicio (2 instancias) ejecuta las consultas de los notebooks en un segundo Almacén de datos Snowflake con 32 instancias: 32 GB/s. Los dos dejan la Concurrencia por instancia en 8 y la Suspensión automática tras en 60 s.

En una hora normal cuesta unos 7580 $ al mes: 140 $ los Servidores de aplicaciones, 140 $ el Microservicio, 7300 $ el almacén de los analistas y 0 $ el de los notebooks. En esa hora no llegó ninguna consulta de exploración, así que el segundo almacén siguió suspendido y no facturó nada. El p95 es de unos 417 ms, sin errores.

"Las 9 de la mañana: cuatro veces las consultas durante 40 s" lleva unos 4,6 GB/s al almacén de los analistas: 5 GB/s los escanean a un 91 % más o menos. El p95 es de unos 632 ms, sin errores. Con 4 instancias, la misma avalancha llega a un p95 de unos 6,2 s; con 2, a unos 50 s.

"El equipo de ciencia de datos se pone a explorar todo el histórico" manda unos 29 GB/s de exploración al almacén de los notebooks, que escanea 32 GB/s, así que sigue el ritmo y cada consulta de exploración termina muy dentro del tiempo límite de 120 s. El almacén de los analistas no las ve: el p95 sigue en unos 417 ms, sin errores. Las consultas de exploración tardan unos 60 s cada una, pero son menos del 4 % de las consultas, así que quedan por encima del p95.

El presupuesto se mide en una hora normal. Mientras el almacén de los notebooks trabaja, factura como cualquier otro: en las pruebas ocultas el mismo diseño marca un ritmo mensual de unos 43 400 $ y 53 900 $ (durante la avalancha también llega una consulta de exploración suelta). La suspensión automática es lo que hace que ese coste sea solo el de las horas de trabajo.

¿Por qué no un único almacén compartido con más clústeres? Cada escaneo de 60 GB retiene las consultas de los analistas que vienen detrás. Con 5 instancias falla incluso la avalancha (p95 de unos 12,3 s), porque le cae la consulta de exploración suelta de un día normal. Con 10 instancias, la prueba de exploración rechaza más o menos el 31,9 % de las consultas; con 36 instancias, cuesta unos 52 840 $ y la prueba de exploración aún llega a un p95 de unos 8,1 s. Los analistas mantienen ocupado todo el día un almacén compartido, así que todos sus clústeres facturan todo el día, mientras que un almacén aparte para los notebooks solo factura mientras exploran.

Para el reto de coste, los diseños baratos que encontró la búsqueda cambian el almacén de los analistas por otro producto, algo que el enunciado descarta (la empresa usa Snowflake), así que este problema no tiene reto.

## Por qué falla el primer intento

Un primer intento habitual conecta el Servidor de aplicaciones y el Microservicio a un único Almacén de datos Snowflake con 1 instancia. Cuesta unos 1740 $ y ya falla "Una hora normal de trabajo": llegan unos 1,14 GB/s a un clúster que escanea 1 GB/s, la cola crece toda la hora y el p95 llega a unos 11,4 s.

A las 9 de la mañana llegan unos 4,6 GB/s a 1 GB/s. La cola se llena en los primeros segundos de la avalancha y el almacén rechaza lo que no cabe: falla más o menos el 68,5 % de las consultas, con un p95 de unos 66 s.

Cuando los notebooks se ponen a explorar, cada consulta de exploración pone un minuto de escaneo por delante de todos. Las consultas de los analistas esperan detrás, la cola se llena y el almacén rechaza lo que no cabe: falla más o menos el 97,6 % de las consultas, con un p95 de unos 61 s.

La idea es esta: la capacidad de un almacén son los GB que escanea por segundo, clústeres × velocidad de escaneo, y se dimensiona para la avalancha, no para la media. Y una carga que escanea todo el histórico no debe ir con otra que necesita respuestas en segundos: dale su propio almacén, y deja que la suspensión automática lo haga barato.

## Lo que el simulador deja fuera

Los almacenes multiclúster reales añaden y quitan clústeres según crece y baja la cola (entre un mínimo y un máximo); aquí los clústeres son fijos, y la tarjeta los factura todos mientras corre cualquier consulta. Reanudar un almacén suspendido tarda unos segundos en la vida real; aquí es instantáneo. Aquí las consultas de un clúster solo reparten su velocidad de escaneo, por orden de llegada; las reales también reparten CPU y memoria, y un planificador las intercala. Un tamaño de almacén mayor (una Velocidad de escaneo por consulta más alta) cuesta lo mismo que los clústeres a los que sustituye, y por eso el enunciado fija el tamaño. No se simulan los tiempos límite de la cola, las reglas de enrutado por usuario ni la corrección de los propios datos: separar las cargas se dibuja como cableado. El precio de Snowflake es la cifra didáctica del registro (2 $ por crédito), y el tiempo está acortado: la avalancha dura 40 s y la exploración 2 minutos.

## Para probar

- Pon el almacén de los analistas en 2 instancias y ejecuta la avalancha de las 9. Mira cómo crece la cola segundo a segundo.
- Conecta el Microservicio al almacén de los analistas, con 10 instancias, y ejecuta la prueba de exploración.
- Pon la Concurrencia por instancia del primer intento en 32 y ejecuta "Una hora normal de trabajo". ¿Por qué no ayuda?
