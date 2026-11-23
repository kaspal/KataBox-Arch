# Solución comentada

## Adónde va la carga

Una consulta de un recuadro que la Caché de resultados no puede responder cuesta unos 0,8 s de un hueco del almacén: 300 ms de sobrecarga más 0,5 GB a 1 GB/s. Un clúster tiene 8 huecos, así que termina unas 10 consultas así por segundo.

En una hora normal, 20 cargas de recuadro por segundo con un 60 % respondido por la Caché de resultados dejan 8 consultas completas por segundo, 8 × 0,8 = 6,4 huecos ocupados de 8. Un clúster aguanta justo. A las 9 de la mañana el tráfico es diez veces mayor: 80 consultas completas por segundo necesitan unos 64 huecos, u 8 clústeres, y en Snowflake cada clúster cuesta 1460 $ al mes mientras funciona.

Pero la avalancha no son 200 preguntas distintas por segundo. Son los mismos 400 recuadros, pedidos una y otra vez. Una caché delante, con el recuadro como clave y un TTL de 600 s, pregunta al almacén unas 400 / 600 ≈ 0,7 veces por segundo cuando está caliente, sean 20 cargas por segundo o 200. El arranque en frío es el único momento difícil: como mucho 400 consultas, una por recuadro, si las peticiones de un recuadro que falta esperan a una sola consulta en lugar de lanzar cada una la suya.

## El diseño de referencia

Un AWS ALB envía el tráfico a 2 Servidores de aplicaciones Go. Estos leen los recuadros de una Caché Redis con un TTL de 600 s y "Agrupar fallos (protección de estampida)" activado; en un fallo consultan un Almacén de datos Snowflake con sus valores por defecto (1 clúster, Concurrencia por instancia 8, Caché de resultados activada, Suspensión automática tras 60 s). En una hora normal cuesta unos 1053 $ al mes: 852 $ el almacén, 140 $ los Servidores de aplicaciones, 36 $ Redis y 25 $ el ALB. El almacén se cobra solo por los segundos que funciona, y con la caché delante pasa largos ratos parado. El p95 es de unos 18,6 ms, sin errores.

"Las 9 de la mañana: cachés vacías y luego diez veces el tráfico durante 40 s" empieza con Redis vacío y multiplica el tráfico por diez a partir del segundo 5. Las primeras cargas de cada recuadro fallan juntas; la protección de estampida lanza una consulta por recuadro, la Caché de resultados responde el 60 % de ellas en unos 30 ms, y las demás esperan los 8 huecos durante unos segundos. Algunos segundos de ese arranque tienen un p95 de más de 2 s, pero el p95 de la prueba es de unos 42,8 ms, sin errores. El almacén funciona más, así que esta prueba cobra unos 1661 $.

"El almacén de datos se ralentiza" añade 3 s a cada consulta que recorre datos, desde el segundo 20 y durante 40 segundos. Solo los pocos fallos de caché llegan al almacén, y su Caché de resultados responde algunos sin recorrer nada. El p95 es de unos 20,2 ms, sin errores.

Cada pieza hace falta. Sin la protección de estampida, el arranque en frío lanza una consulta por cada carga de un recuadro que falta, y la avalancha falla con alrededor de un 2,5 % de errores. Con la Caché de resultados apagada, todas esas consultas recorren datos, y la avalancha falla con alrededor de un 23 % de errores.

El reto de coste no encontró nada por debajo del 80 % del coste de la referencia (su mejor prueba se quedó en el 91 %), así que este problema no tiene reto de coste: el almacén es casi toda la factura, y un clúster es lo mínimo que puedes tener.

## Por qué falla el primer intento

Un primer intento habitual es el mismo ALB y 2 Servidores de aplicaciones Go consultando directamente un clúster de Snowflake, con la Caché de resultados activada y sin caché delante. Cuesta unos 1625 $ al mes, porque el almacén está ocupado el 78 % del tiempo y por eso se cobra casi todo el tiempo. Pasa "Una hora normal" con un p95 de unos 1037 ms: el 40 % de las consultas que recorren datos tardan 0,8 s, más algo de cola.

A las 9 de la mañana necesita 64 huecos y tiene 8. La cola se llena en segundos, las consultas esperan más que los 5 s del cliente o se rechazan, y fallan alrededor del 81 % de las cargas. La Caché de resultados no lo salva: una consulta repetida es rápida, pero sigue ocupando un hueco y un sitio en la cola, y el 40 % de la avalancha sigue recorriendo datos.

Cuando el almacén se ralentiza, cada consulta que recorre datos tarda unos 3,8 s. 8 por segundo necesitan ahora unos 30 huecos en lugar de 6,4; la cola crece desde los primeros segundos, y fallan alrededor del 39 % de las cargas.

Añadir clústeres es la salida cara. Aquí, 8 clústeres ocupados durante una prueba cobran unos 8 × 1460 $ = 11 680 $ al mes, muy por encima del presupuesto; incluso un almacén real que se suspende solo tendría los 8 en marcha cada mañana para responder los mismos 400 recuadros. La idea es esta: un panel son unas pocas preguntas hechas muchas veces. Responde cada una una vez, guarda la respuesta tanto como permitan los datos y deja al almacén el trabajo que de verdad es nuevo.

## Lo que el simulador deja fuera

La Caché de resultados del almacén se modela como una parte fija de las consultas (el 60 %) respondida rápido; una real depende del texto exacto de la consulta, del rol del usuario y de si las tablas cambiaron desde entonces, y la carga de cada hora la vacía. El escalado automático de clústeres de Snowflake (añadir clústeres cuando las consultas hacen cola) no se simula: Instancias es un número fijo de clústeres. Aquí subir Concurrencia por instancia añade huecos gratis, mientras que en un almacén real las consultas compartirían el mismo cómputo y cada una iría más lenta. No se cobran el mínimo de 60 s que factura Snowflake cada vez que un clúster se reanuda ni el coste de cargar los datos cada hora. Los precios son los precios didácticos del registro. La avalancha se reduce a 40 segundos.

## Para probar

- Desactiva "Agrupar fallos (protección de estampida)" en la Caché Redis y ejecuta la prueba de las 9 de la mañana. Observa la cola del almacén en los primeros segundos.
- Desactiva la Caché de resultados del almacén y vuelve a ejecutar la prueba de las 9 de la mañana.
- Quita la Caché Redis y dale al almacén más clústeres hasta que pase la prueba de las 9 de la mañana. Compara la factura con la de la referencia.
