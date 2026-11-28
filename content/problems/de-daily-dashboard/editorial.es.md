# Solución comentada

## Adónde va la carga

Una consulta de un recuadro que la Caché de resultados no puede responder cuesta 300 ms de sobrecarga más 0,5 GB de escaneo. Un clúster tiene 8 huecos, pero escanea 1 GB/s en total: las instancias de un almacén reparten su velocidad de escaneo entre las consultas que se ejecutan a la vez, así que ocho escaneos juntos reciben cada uno una octava parte. Un clúster termina como mucho 2 consultas con escaneo por segundo, tenga los huecos que tenga.

En una hora normal, 20 cargas de recuadro por segundo con un 60 % respondido por la Caché de resultados dejan 8 consultas completas por segundo: 4 GB/s de escaneo, el trabajo de cuatro clústeres. A las 9 de la mañana el tráfico es diez veces mayor: 80 consultas completas por segundo necesitan 40 GB/s, o 40 clústeres, y en Snowflake cada clúster cuesta 1460 $ al mes mientras funciona.

Pero la avalancha no son 200 preguntas distintas por segundo. Son los mismos 400 recuadros, pedidos una y otra vez. Una caché delante, con el recuadro como clave y un TTL de 600 s, pregunta al almacén unas 400 / 600 ≈ 0,7 veces por segundo cuando está caliente, sean 20 cargas por segundo o 200. El arranque en frío es el único momento difícil: como mucho 400 consultas, una por recuadro, si las peticiones de un recuadro que falta esperan a una sola consulta en lugar de lanzar cada una la suya. Alrededor del 40 % de ellas escanean, hasta 80 GB en los primeros segundos. Un clúster necesita más de un minuto para eso, mucho más que los 5 s del cliente; cinco clústeres escanean 5 GB/s y se ponen al día mientras los recuadros populares ya están en caché.

## El diseño de referencia

Un AWS ALB envía el tráfico a 2 Servidores de aplicaciones Go. Estos leen los recuadros de una Caché Redis con un TTL de 600 s y "Agrupar fallos (protección de estampida)" activado; en un fallo consultan un Almacén de datos Snowflake con 5 clústeres (Instancias 5) y sus demás valores por defecto (Concurrencia por instancia 8, Caché de resultados activada, Suspensión automática tras 60 s). En una hora normal cuesta unos 4448 $ al mes: 4258 $ el almacén, 140 $ los Servidores de aplicaciones, 24 $ Redis y 25 $ el ALB. El almacén se cobra solo por los segundos que funciona, y con la caché delante pasa largos ratos parado. El p95 es de unos 18,6 ms, sin errores.

"Las 9 de la mañana: cachés vacías y luego diez veces el tráfico durante 40 s" empieza con Redis vacío y multiplica el tráfico por diez a partir del segundo 5. Las primeras cargas de cada recuadro fallan juntas; la protección de estampida lanza una consulta por recuadro, la Caché de resultados responde el 60 % de ellas en unos 30 ms, y las demás se reparten los 5 GB/s de los cinco clústeres durante unos segundos. El p95 de la prueba es de unos 37,2 ms, sin errores. El almacén funciona más, así que esta prueba cobra unos 7489 $.

"El almacén de datos se ralentiza" añade 3 s a cada consulta que recorre datos, desde el segundo 20 y durante 40 segundos. Solo los pocos fallos de caché llegan al almacén, y su Caché de resultados responde algunos sin recorrer nada. El p95 es de unos 20,2 ms, sin errores.

La caché delante y la Caché de resultados hacen falta las dos. Con la Caché de resultados apagada, todas las consultas del arranque en frío recorren datos, y la avalancha falla con alrededor de un 25 % de errores. Con un clúster en lugar de cinco, los escaneos del arranque en frío hacen cola unos detrás de otros y la avalancha falla con alrededor de un 23 % de errores; con cuatro, falla alrededor del 0,24 % de las cargas, más de lo que permite la disponibilidad del 99,9 %. La protección de estampida es un seguro barato: con cinco clústeres la avalancha pasa también sin ella, pero entonces cada carga de un recuadro que falta lanza su propia consulta.

La única opción más barata que encuentra el reto de coste lleva las tablas a ClickHouse, algo que el enunciado descarta (los datos viven en Snowflake), así que este problema no tiene reto de coste: el almacén es casi toda la factura.

## Por qué falla el primer intento

Un primer intento habitual es el mismo ALB y 2 Servidores de aplicaciones Go consultando directamente un clúster de Snowflake, con la Caché de resultados activada y sin caché delante. Cuesta unos 1625 $ al mes, porque el almacén está ocupado todo el tiempo. Falla incluso "Una hora normal": 8 consultas con escaneo por segundo piden 4 GB/s a un clúster que escanea 1 GB/s, la cola crece desde los primeros segundos, las consultas esperan más que los 5 s del cliente o se rechazan, y fallan alrededor del 85 % de las cargas (p95 de unos 4957 ms).

A las 9 de la mañana necesita 40 GB/s y tiene 1, y fallan alrededor del 97 % de las cargas. La Caché de resultados no lo salva: una consulta repetida es rápida, pero sigue ocupando un hueco y un sitio en la cola, y el 40 % de la avalancha sigue recorriendo datos. Cuando el almacén se ralentiza, fallan alrededor del 88 % de las cargas.

Añadir clústeres es la salida cara. Aquí, 40 clústeres ocupados durante la avalancha cobrarían unos 40 × 1460 $ = 58 400 $ al mes, muy por encima del presupuesto; incluso un almacén real que se suspende solo tendría los 40 en marcha cada mañana para responder los mismos 400 recuadros. La idea es esta: un panel son unas pocas preguntas hechas muchas veces. Responde cada una una vez, guarda la respuesta tanto como permitan los datos y deja al almacén el trabajo que de verdad es nuevo.

## Lo que el simulador deja fuera

La Caché de resultados del almacén se modela como una parte fija de las consultas (el 60 %) respondida rápido; una real depende del texto exacto de la consulta, del rol del usuario y de si las tablas cambiaron desde entonces, y la carga de cada hora la vacía. El escalado automático de clústeres de Snowflake (añadir clústeres cuando las consultas hacen cola) no se simula: Instancias es un número fijo de clústeres. Subir Concurrencia por instancia añade huecos pero no escaneo: las consultas de un clúster se reparten su velocidad de escaneo, como en un almacén real. No se cobran el mínimo de 60 s que factura Snowflake cada vez que un clúster se reanuda ni el coste de cargar los datos cada hora. Los precios son los precios didácticos del registro. La avalancha se reduce a 40 segundos.

## Para probar

- Pon Instancias del almacén a 1, luego a 4, y ejecuta la prueba de las 9 de la mañana. Observa la cola del almacén en los primeros segundos.
- Desactiva "Agrupar fallos (protección de estampida)" en la Caché Redis y ejecuta la prueba de las 9 de la mañana. Cuenta las consultas que llegan al almacén.
- Desactiva la Caché de resultados del almacén y vuelve a ejecutar la prueba de las 9 de la mañana.
- Quita la Caché Redis y dale al almacén más clústeres hasta que pase la prueba de las 9 de la mañana. Compara la factura con la de la referencia.
