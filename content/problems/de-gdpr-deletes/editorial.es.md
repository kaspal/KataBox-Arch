# Editorial

## Dónde va la carga

Cada petición de borrado sigue el mismo camino: el Servidor de aplicaciones la publica, un Worker la toma y escribe en tres almacenes por turno. El Worker mantiene su hueco todo ese tiempo: 100 ms de trabajo propio, unos 50 ms para la Tabla lakehouse, unos 300 ms para el Almacén de datos y unos pocos milisegundos para DynamoDB, así que unos 0,46 s por borrado. A 20 por segundo eso mantiene ocupados unos 20 × 0,46 ≈ 9 huecos de Worker, y 20 × 0,3 = 6 huecos del Almacén de datos.

Dos números marcan lo rápido que se terminan los borrados: los huecos de Worker (borrados en curso a la vez ÷ tiempo por borrado) y los huecos del Almacén de datos (cada instancia termina como mucho 8 ÷ 0,3 s ≈ 27 borrados por segundo). El más lento de los dos es la velocidad del pipeline. Lo que llega por encima no se rechaza; espera en la cola, y el plazo trata de cuánto espera.

Las pruebas ocultas aprietan cada número:
- **Una noticia sobre privacidad se hace viral** triplica las peticiones a 60 por segundo durante 40 segundos, desde el segundo 10. Eso necesita unos 60 × 0,3 = 18 huecos del almacén y 60 × 0,46 ≈ 28 huecos de Worker.
- **Las reescrituras de la tabla del lago se ralentizan** suma 3 s a cada escritura en la Tabla lakehouse durante 50 segundos, desde el segundo 15. Nada más cambia, pero cada borrado ocupa ahora su hueco de Worker unos 3,5 s: a 20 por segundo, son unos 70 huecos ocupados.

## El diseño de referencia

El cliente web llama a los Servidores de aplicaciones Go (2 instancias, el valor por defecto), que publican en una Cola Amazon SQS con «Reintentos máximos» a 8 y una Cola de mensajes fallidos conectada desde ella con Publicar. 8 Workers Celery con la concurrencia por defecto de 8 (64 huecos) la consumen y escriben en una Tabla lakehouse Apache Iceberg, un Almacén de datos Snowflake con 2 instancias (16 huecos) y un Almacén NoSQL DynamoDB. Cuesta unos 3947 $ al mes: 2920 $ de Snowflake, 560 $ de los Workers, 236 $ del almacenamiento de la tabla del lago, 140 $ de los Servidores de aplicaciones, 60 $ de SQS y 31 $ de DynamoDB. Un día normal tiene un p99 de unos 1,2 s; los Workers están ocupados un 16 % y el almacén un 40 %.

Durante la avalancha, 60 peticiones por segundo se encuentran con un almacén que termina unas 50. La cola crece unas 10 por segundo durante 40 segundos, unas 400 peticiones, y luego se vacía a unas 30 por segundo cuando acaba la avalancha. Las últimas peticiones de la avalancha esperan unos 10 s: el p99 es de unos 11,6 s, muy dentro de los 30 s. El diseño no tiene sitio para toda la avalancha, y no le hace falta: necesita un atasco que se vacíe antes de envejecer.

Cuando el lago se ralentiza, los 64 huecos de Worker terminan unos 64 ÷ 3,5 ≈ 18 borrados por segundo, cerca de los 20 que llegan. Se forma un pequeño atasco que se despeja: el p99 es de unos 7,5 s.

Los reintentos y la Cola de mensajes fallidos están para los fallos: ninguna prueba oculta hace fallar un almacén, y ninguna petición de la referencia se reintenta. Con 8 reintentos una petición sigue intentándolo unos 2 minutos, lo bastante para aguantar el reinicio de un almacén, y lo que aún falle espera en la Cola de mensajes fallidos en vez de perderse. Un borrado se puede reintentar sin peligro: borrar a un usuario que ya no está en un almacén no cambia nada.

No hay reto de coste en este problema: la búsqueda encontró un diseño al 91 % del coste de la referencia (3597 $), no por debajo del 80 % que necesita un reto.

## Por qué falla el primer intento

Un primer intento habitual es el mismo camino con los valores por defecto: 2 Workers Celery (16 huecos), una instancia de Snowflake (8 huecos), 3 reintentos y sin Cola de mensajes fallidos. Cuesta unos 2067 $ al mes y pasa «Un día normal» con un p99 de unos 1,4 s. Pero ya va justo: los Workers están ocupados un 69 % y el almacén un 81 %.

En la avalancha, la única instancia del almacén termina unos 25 borrados por segundo mientras llegan 60. La cola crece unas 35 por segundo, unas 1400 peticiones cuando acaba la avalancha, y después se vacía a solo unas 5 por segundo. Las peticiones esperan más de un minuto, y el p99 es de unos 62,5 s. Añadir una segunda instancia al almacén no basta por sí solo: 16 huecos de Worker no dan para los 28 borrados en curso que necesita la avalancha, y el p99 sigue en unos 41 s.

Cuando el lago se ralentiza, cada borrado ocupa un hueco de Worker unos 3,5 s, y 16 huecos terminan menos de 5 por segundo mientras llegan 20. Nada falla y nada se pierde: las peticiones solo esperan, y el p99 es de unos 38 s. Ocho Workers por sí solos arreglan esta prueba (p99 de unos 8,6 s), pero no la avalancha.

La idea es esta: un plazo es un problema de rendimiento. Busca la etapa más lenta (los huecos de Worker cuando un almacén se ralentiza, el almacén de datos cuando llega una avalancha), dale sitio suficiente para que un atasco se vacíe con holgura dentro del plazo, y recuerda que un almacén lento ocupa los huecos que necesitan los borrados de los demás almacenes. Un diseño con 4 Workers y 2 instancias del almacén también pasa, pero el lago lento lleva su p99 a unos 27 s, con poco margen.

## Lo que el simulador deja fuera

No se borra nada de verdad. Un borrado son tres llamadas de escritura con un coste fijo; el simulador no busca las filas del usuario, no reescribe los ficheros de la tabla del lago y no comprueba que los datos hayan desaparecido. En una tabla Iceberg, Delta o Hudi real, un borrado reescribe cada fichero que contiene alguna fila del usuario (o escribe ficheros de borrado que se fusionan más tarde), y las instantáneas antiguas siguen guardando los datos hasta que caducan y se limpian. Esos costes dependen de la organización de la tabla y no se modelan; la prueba del lago lento los representa como tiempo extra por escritura.

Los pipelines de borrado reales agrupan el trabajo: reúnen las peticiones de un día y ejecutan un solo borrado por tabla para todas, lo que sale mucho más barato que una reescritura por usuario. El simulador no agrupa escrituras, así que este problema dimensiona el camino petición a petición. Las copias de seguridad, los logs, las cachés y las copias que tienen los socios también guardan datos del usuario, y ninguna está aquí.

El Worker escribe en los tres almacenes uno detrás de otro. Un diseño real podría dar a cada almacén su propia cola y sus Workers, para que un lago lento no retrase los borrados del almacén de datos y de DynamoDB; el simulador ejecuta las llamadas de un llamador en orden, así que aquí un almacén lento frena todos los borrados.

El plazo está a escala: 30 segundos representan 24 horas, y las pruebas duran uno o dos minutos. A lo largo de un día real se cumple la misma cuenta con números más grandes: un atasco que crece durante horas tiene que vaciarse antes de que la petición más antigua cumpla 24 horas.

## Cosas que probar

- Baja a 4 Workers y vuelve a ejecutar el lago lento; luego prueba un grupo de autoescalado para los Workers y mira cuánto tarda en reaccionar.
- Sube la «Concurrencia por instancia» del Almacén de datos en vez de añadir una instancia, y compara el coste.
- Da a los 2 Workers por defecto una «Concurrencia por instancia» de 32 en vez de añadir instancias, y compara el coste: sus huecos pasan casi todo el tiempo esperando a los almacenes.
