# Solución comentada

## Adónde va la carga

Para cada mensaje pasan dos cosas muy distintas. La respuesta del modelo tarda segundos: unos 1,1 s antes del primer token y después unos 300 tokens a 100 tokens por segundo, así que más o menos de 4 a 5 s en la mediana y unos 7 s en el p95. La escritura de la traza es pequeña, pero un almacén de datos es lento con las escrituras pequeñas: cada una ocupa uno de sus 8 huecos unos 300 ms.

En el pico de 5 mensajes por segundo, las trazas necesitan 5 × 0,3 = 1,5 de los 8 huecos del almacén, un 19 % más o menos. El lanzamiento lo multiplica por 4: 20 trazas por segundo necesitan 6 huecos, el 75 %. La cuenta del modelo no es el límite: 20 mensajes por segundo de unos 1100 tokens son 1 320 000 tokens por minuto, por debajo de los 2 000 000 permitidos.

En la ralentización es donde importa la forma del diseño. Con 8 s más, cada escritura tarda unos 8,3 s, así que 8 huecos terminan unas 8 / 8,3 ≈ 1 traza por segundo mientras llegan 5. El almacén se queda atrás unas 4 trazas cada segundo durante 40 segundos. La pregunta es quién espera a esa cola.

## El diseño de referencia

Un Cliente web con un Tiempo límite de 30 000 ms envía los mensajes a un AWS ALB, que los reparte entre 4 Servidores de aplicaciones Go. Estos llaman a una API de Claude (Límite de peticiones por minuto 4000, Límite de tokens por minuto 2 000 000), responden al chat y publican su traza en un Stream Apache Kafka con 12 particiones. Un Procesador de streams Apache Flink con 2 instancias y una Concurrencia por instancia de 8 consume el Stream y escribe cada traza en un Almacén de datos Snowflake con sus ajustes por defecto (1 instancia, 8 huecos). Cuesta unos 64 565 $ al mes, y los tokens son casi todo: 62 440 $ la API de Claude, 1387 $ Snowflake (se cobra mientras funciona, que en un día con tráfico es casi siempre), 280 $ los Servidores de aplicaciones, 280 $ Flink, 153 $ Kafka y 25 $ el ALB. Un día normal tiene un p95 de unos 6,9 s y ningún error.

«Día de lanzamiento» multiplica el tráfico por 4 a partir del segundo 30, durante 2 minutos, en una ejecución de 3 minutos. Un lanzamiento real dura horas; la prueba dura lo bastante para que se vea cualquier cola. El almacén está ocupado un 75 % con las trazas, las 12 particiones permiten 12 escrituras a la vez y los Servidores de aplicaciones tienen margen. No hay errores y el p95 es de unos 7,4 s.

«El almacén de datos se ralentiza» suma 8 s a cada escritura en el almacén a partir del segundo 20, durante 40 segundos, en una ejecución de 90 segundos. Los Servidores de aplicaciones no lo notan: publican la traza en Kafka en unos pocos milisegundos y responden al chat. Las escrituras de Flink tardan ahora unos 8,3 s cada una, y con 12 como mucho en curso las trazas se acumulan en el Stream, unas 140 al final de la ralentización (una cuenta aproximada con los números de arriba). No se pierde ninguna: se quedan en Kafka hasta que Flink las escribe, y la cola se vacía en segundos cuando el almacén vuelve a ir rápido. Los paneles van atrasados más o menos un minuto, y eso se lo pueden permitir. El chat tiene un p95 de unos 7,2 s y ningún error.

No hay reto de coste en este problema. Los tokens son un 97 % de la factura y los marca el tráfico; el diseño más barato que se encontró y que supera todas las pruebas cuesta lo mismo que la referencia.

## Por qué falla el primer intento

Un primer intento habitual es el mismo Cliente web, ALB, Servidores de aplicaciones y API de Claude, con los Servidores de aplicaciones escribiendo ellos mismos cada traza en el mismo almacén Snowflake, justo después de que responda el modelo. Cuesta unos 64 132 $, algo menos sin Kafka ni Flink, y supera «Un día normal» con un p95 de unos 7,4 s: los 300 ms de la escritura se suman a cada respuesta, pero hay sitio para ellos. Incluso supera «Día de lanzamiento», con un p95 de unos 7,7 s, porque el almacén todavía da abasto.

La ralentización lo rompe: un p95 de unos 26,7 s y un 13,8 % de errores. Cada mensaje espera ahora a la escritura de su propia traza, y las escrituras hacen cola tras los 8 huecos lentos del almacén. En 10 segundos las respuestas tardan de 15 a 25 s, aunque el modelo va igual de rápido que antes. A partir de unos 20 segundos de ralentización, los mensajes empiezan a llegar al Tiempo límite de 30 s del cliente y fallan, hasta que el almacén vuelve a ir rápido y su cola se vacía.

Un almacén más grande no lo arregla. Con 4 instancias de Snowflake (32 huecos), el primer intento cuesta unos 68 293 $ y sigue fallando la ralentización con un p95 de unos 17,3 s: 32 huecos a 8,3 s terminan unas 3,9 trazas por segundo, todavía menos de las que llegan, y cada respuesta sigue esperando al menos 8,3 s a su propia escritura.

La idea es esta: el trabajo que el usuario no necesita ver no debería estar en su camino. Una traza es para los analistas, así que guárdala a salvo (una publicación en un Stream) y deja que un consumidor la lleve al almacén al ritmo del almacén. Así, un almacén lento te cuesta paneles al día durante un minuto, no mensajes fallidos.

## Lo que el simulador deja fuera

No se simula la calidad de las respuestas, ni lo que lleva una traza: su tamaño, sus campos o la cifra de coste que contiene. Las consultas de los propios paneles no forman parte del tráfico, así que nunca compiten con las escrituras de trazas por los huecos del almacén. Un trabajo real de Flink (o Snowpipe Streaming) envía las trazas al almacén por lotes, y eso hace mucho más barato cargar cada traza; aquí cada traza es una escritura de 300 ms en los dos diseños, así que el almacén cuesta lo mismo. No se comprueba la entrega exactamente una vez, y ninguna prueba llega a la retención de Kafka (7 días). El streaming de la respuesta se modela solo como tiempos, y el p95 se mide cuando responde el Servidor de aplicaciones. Los precios de los tokens son las cifras del registro.

## Para probar

- Pon las Particiones del Stream en 2 y ejecuta «Día de lanzamiento». Mira la cola del Stream: 2 escrituras en curso de 300 ms cada una mueven menos trazas de las que llegan, mientras el chat sigue yendo rápido.
- Dale 4 instancias al Snowflake del primer intento y ejecuta la ralentización. Compara su p95 con el de la referencia.
- En la referencia, mata la tarjeta de Snowflake durante 30 s y mira cómo el Stream guarda las trazas hasta que vuelve.
