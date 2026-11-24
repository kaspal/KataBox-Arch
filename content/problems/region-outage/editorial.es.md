# Solución comentada

## Adónde va la carga

Con 6000 peticiones/s, unas 5400 son lecturas y 600 son escrituras; repartidas entre tres Regiones, cada una atiende unas 2000 por segundo.

Una lectura local tarda unos 10 ms: los 5 ms de Go más los 5 ms de PostgreSQL. Una escritura desde una Región sin el primario cruza dos veces, 70 ms en cada sentido, unos 150 ms en total. Cruzan unas 400 escrituras por segundo, cerca del 6,7 % de las peticiones, más del 5 %, así que el p95 es una de ellas: unos 150 ms, dentro de 250 ms. Cada una ocupa un hueco de App Server todo ese tiempo, así que esas Regiones están el doble de ocupadas.

La capacidad es fácil. Lo difícil es que una Región entera puede fallar a la vez.

## El diseño de referencia

DNS Route 53, con un TTL de 30 s, reparte a los clientes entre tres Regiones: us-east, eu-west y ap-south. Cada una tiene un AWS ALB, un App Server Go con 2 instancias y un miembro PostgreSQL. El primario está en us-east, y los miembros de eu-west y ap-south son réplicas conectadas a él con enlaces Replicate. Cuesta unos 1338 $ al mes, de los que DNS son unos 62 $, con un p95 de unos 155 ms.

En un día normal, cerca del 8 % de las lecturas son obsoletas, porque las réplicas van 50 ms por detrás; el problema solo juzga la obsolescencia durante el aislamiento.

«Se apaga la región del primario» pierde la Región entera que tiene el primario de la base de datos en el segundo 30, para siempre, en una ejecución de 120 segundos. Admite un 8 % de errores, un 92 % de disponibilidad y un p95 de 1 s. La prueba encuentra esa Región por sí sola. DNS sigue enviando allí un tercio del tráfico hasta que los health checks lo detectan, entre 2 y 4 segundos después, y esas peticiones fallan, junto con las que estaban en curso. Las escrituras fallan hasta que una réplica pasa a primario, tras el tiempo de conmutación por defecto de PostgreSQL, 30 segundos. En total falla cerca del 3 % de las peticiones, y el p95 de las respondidas es de unos 43 ms.

«La región del primario queda aislada 20 s» aísla esa Región a partir del segundo 20, durante 20 segundos: sigue funcionando, pero nada cruza su frontera. Tiene los mismos límites, más un máximo de un 10 % de lecturas obsoletas. Las peticiones que se envían allí fallan hasta que el enrutado detecta el aislamiento, y entonces DNS manda a todo el mundo a las otras dos Regiones. Las lecturas siguen saliendo de sus réplicas. El primario está vivo, solo fuera de alcance, así que no se promueve nada, y las escrituras fallan hasta que se cura el aislamiento. Falla cerca del 4,3 % de las peticiones, y cerca del 6,6 % de las lecturas son obsoletas.

## Por qué falla el primer intento

Un primer intento habitual es una sola Región, us-east, con dos Zonas de disponibilidad, cada una con un App Server Go de 3 instancias y un miembro PostgreSQL: el primario en una y su réplica en la otra. Cuesta unos 1028 $ al mes y supera «Un día normal» con un p95 de unos 32 ms.

Cuando se apaga su Región no queda nada: cerca de un 75 % de errores, todas las peticiones desde el segundo 30. El aislamiento es igual de malo. Los clientes y el DNS están fuera de la Región, así que durante 20 de 60 segundos no entra ninguna petición: cerca de un 33 % de errores. Las zonas te protegen de que falle un centro de datos, no de que falle una Región.

## Lo que el simulador deja fuera

El simulador no guarda en caché las respuestas de DNS al enrutar: cada petición sigue la respuesta del momento. Los clientes reales guardan la respuesta durante el TTL, así que hacen muchas menos consultas, pero también siguen yendo a una Región caída hasta que el TTL se agota. La factura sí tiene en cuenta esa caché: solo alrededor de 1 petición de cada 100 se cobra como consulta. No se simula escribir en todas las Regiones a la vez (varios primarios con resolución de conflictos), y los datos que viajan entre Regiones no se facturan.

## Para probar

- Convierte el miembro de eu-west en el primario y vuelve a ejecutar las dos pruebas ocultas. Siguen al primario esté donde esté.
- Borra ap-south y ejecuta «Se apaga la región del primario». Entonces queda una sola Región para llevar todo el tráfico.
- Añade una Cache Redis en cada Región y observa las lecturas obsoletas en la prueba de aislamiento.
