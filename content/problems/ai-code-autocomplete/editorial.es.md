# Solución comentada

## Adónde va la carga

Los Servidores de aplicaciones y el balanceador de carga hacen poco: pasan cada pulsación y esperan. El tiempo se va en las GPU, y el presupuesto es justo.

En una GPU ociosa, una sugerencia necesita 15 ms más su prompt leído a 50 000 tokens por segundo, y después sus tokens a 500 por segundo. Una petición típica tarda 15 + 800 / 50 000 s = 31 ms hasta su primer token y 10 / 500 s = 20 ms en escribirse: unos 51 ms. Una larga (2000 tokens de código y una sugerencia de 30 tokens) tarda 55 + 60 = 115 ms. Eso ya gasta la mayor parte del presupuesto de 150 ms, con la GPU sin hacer nada más.

Ahora llena el lote. Con B de los 64 huecos ocupados, cada sugerencia se escribe a 500 × (1 − 0,5 × B ÷ 64) tokens por segundo. Con el lote lleno, los 60 ms de escritura de la sugerencia larga pasan a 120 ms, y la petición tarda 175 ms: por encima del presupuesto antes de esperar nada. Y una GPU llena hace esperar un hueco a la siguiente petición. Así que un diseño para este presupuesto necesita GPU que sigan lejos de llenarse, incluso en el peor momento.

El pico es de 1500 peticiones por segundo, y la hora punta, de 6000. A unos 50 ms cada una, son unas 75 sugerencias en curso en el pico y 300 en la hora punta, más en la práctica porque las GPU más ocupadas escriben más despacio. Cuatro GPU tienen 256 huecos.

La otra palanca es mandar menos peticiones a las GPU. Un prefijo idéntico recibe la misma sugerencia, y unos pocos prefijos se escriben muy a menudo. Una Caché Redis de 4 GB guarda unos 4 millones de sugerencias de 1 KB; con un TTL de una hora, conserva los prefijos que vuelven, y un acierto cuesta un milisegundo en vez de un hueco de GPU. En estas ejecuciones responde más o menos 4 de cada 10 peticiones, y en la hora punta se queda con su parte de cuatro veces el tráfico.

## El diseño de referencia

Un Cliente web con un Tiempo límite de 2000 ms, 2 Reintentos, una Espera entre reintentos de 20 ms y la Variación de la espera en Completa (espera aleatoria) envía las pulsaciones a un AWS ALB, que las reparte entre 6 Servidores de aplicaciones Go. Estos leen una Caché Redis con 4 GB de Memoria y un TTL de 3600 s. Si falla, llaman por turnos a tres tarjetas vLLM en GPU, cada una con 2 Instancias (2 GPU), los Huecos de lote por GPU en 64 y las velocidades del modelo que da el enunciado: 6 GPU. Cuesta unos 4921 $ al mes: 4404 $ por las 6 GPU, 420 $ por los Servidores de aplicaciones, 72 $ por la Caché Redis y 25 $ por el ALB.

En un día normal las GPU están ocupadas un 15 % más o menos, el p95 es de unos 106 ms y no hay errores.

«Una hora punta: cuatro veces más tecleo» multiplica el tráfico por cuatro desde el segundo 20, durante un minuto, en una ejecución de 90 segundos (una hora punta real dura una hora; un minuto basta para que se note cualquier cola). Con cuatro veces el tráfico, las GPU están ocupadas un 71 % más o menos, y los Servidores de aplicaciones, un 83 %. Nada espera un hueco, y el p95 sube a unos 127 ms, porque cada sugerencia se escribe algo más despacio en un lote más lleno. No hay errores.

«Se pierde una tarjeta de GPU» detiene la primera tarjeta de vLLM desde el segundo 20 hasta el final de la ejecución. Cuatro GPU y la caché sostienen el pico normal. Durante un momento, hasta que los chequeos de salud ven la tarjeta caída, los Servidores de aplicaciones le siguen mandando un tercio de los fallos de caché; esas llamadas fallan al instante, y el reintento del editor, tras una espera aleatoria de hasta 20 ms, llega a una tarjeta que funciona. Falla más o menos el 0,04 % de las peticiones y el p95 es de unos 109 ms.

Dos tarjetas de 3 GPU también pasan, por el mismo coste (un 0,14 % de errores más o menos al perder una tarjeta, y un p95 de unos 111 ms).

La caché es la parte más barata del diseño y la que más ahorra. Sin ella, las mismas 6 GPU fallan la hora punta con un 23 % de errores más o menos. Ocho GPU sin caché (cuatro tarjetas de 2, unos 6457 $) siguen fallándola, con un 1,2 % de errores más o menos. Nueve GPU sin caché (tres tarjetas de 3) pasan todas las pruebas, pero cuestan unos 7191 $, por encima del presupuesto de 7000 $. Una caché de 72 $ hace el trabajo de tres o más GPU de 734 $ cada una.

Este problema no tiene reto de coste. La búsqueda de un diseño más barato que pase todas las pruebas no encontró nada por debajo del 80 % del coste de la referencia: lo mejor fue un 99 %. Una GPU menos, o una caché más pequeña, falla la hora punta.

## Por qué falla el primer intento

Un primer intento habitual manda cada pulsación al modelo. Una tarjeta vLLM en GPU de 4 Instancias, con los Huecos de lote por GPU en 64 y las velocidades del enunciado, admite 256 sugerencias a la vez, más que las 75 en curso en el pico. Con el mismo cliente, el ALB y 6 Servidores de aplicaciones, y sin caché, cuesta unos 3381 $ al mes y pasa «Un día normal»: las GPU están ocupadas un 38 % más o menos y el p95 es de unos 122 ms, más lento que la referencia porque los lotes van más llenos.

En la hora punta, las 4 GPU reciben 6000 peticiones por segundo. Los lotes más llenos escriben cada sugerencia más despacio, así que cada una ocupa su hueco más tiempo, así que los lotes se llenan: las GPU están ocupadas al 100 %, y las peticiones esperan un hueco. Los Servidores de aplicaciones, cada uno con una petición en espera, se llenan y rechazan el resto. Durante el minuto punta falla casi la mitad de las peticiones, más o menos el 42,8 % en toda la prueba, y el p95 es de unos 1215 ms. El límite son las GPU, no los Servidores de aplicaciones: con 24 Servidores de aplicaciones en vez de 6, la hora punta sigue fallando, con más errores, porque más peticiones llegan a las GPU llenas y esperan allí.

Cuando se pierde la tarjeta, no queda nada: falla cada petición desde el segundo 20, más o menos el 77,8 % de la prueba.

Otros primeros intentos fallan de formas instructivas:

- Una caché delante de dos tarjetas de 2 GPU (unos 3453 $) falla la hora punta con un 15,1 % de errores más o menos. La caché ahorra GPU; no sustituye al margen.
- Seis GPU en una sola tarjeta tienen las GPU de la referencia y las pierden todas a la vez cuando cae la tarjeta.

La idea es esta: en un presupuesto interactivo, el margen de una GPU es latencia. Un lote más lleno es más lento para cada petición que lleva, y uno lleno hace cola, así que dimensiona las GPU para que sigan lejos de llenarse en el momento de más tráfico, en tarjetas iguales para que la pérdida de una deje suficiente. Y no gastes una GPU en una petición que puede responder una caché: un prefijo repetido respondido en un milisegundo es a la vez la respuesta más rápida y la más barata.

## Lo que el simulador deja fuera

La calidad de las sugerencias no se simula: otro modelo, un lote más lleno o una respuesta en caché dan la misma sugerencia para el simulador. La caché de prefijos propia de vLLM (reutilizar el trabajo de la GPU sobre un prefijo compartido dentro del servidor del modelo) y la decodificación especulativa no se simulan; aquí la caché es una Caché Redis aparte con sugerencias terminadas. Los editores reales esperan a una pausa en el tecleo antes de preguntar, y cancelan una petición cuando la siguiente pulsación la deja inútil; nada de eso se simula, así que cada petición llega hasta el final. La memoria de la GPU no se simula: el límite de 64 sugerencias a la vez en una GPU viene del enunciado. La ralentización dentro de un lote es una línea recta, una decisión de modelado. Así llena el simulador la caché. Empieza caliente, como tras una hora de tráfico: está en ella cada prefijo que se escribe al menos una vez en su TTL, tantos como caben en sus 4 GB. Después, cada sugerencia del modelo se guarda en la caché (cache-aside), así que un prefijo que falló y vuelve dentro del TTL es un acierto. En pruebas de 90 segundos esta segunda parte casi no añade nada: los prefijos que vuelven tan pronto son los populares, y ya están en la caché caliente. De un modo u otro, la caché responde alrededor del 38 % de las peticiones. Arrancar un servidor de GPU y cargar el modelo no se simula, y una tarjeta de Inferencia de modelos no puede autoescalarse aquí. Los precios de las GPU y de la caché son las cifras didácticas del registro, no presupuestos reales.

## Para probar

- Quita la caché de la referencia y ejecuta la hora punta. ¿Cuántas GPU hacen falta para pasar sin ella, y cuánto cuestan?
- Dale 6 Instancias a la tarjeta del primer intento y ejecuta «Se pierde una tarjeta de GPU». Después repártelas en tres tarjetas de 2.
- Pon los Huecos de lote por GPU de la referencia en 32 y ejecuta la hora punta. Una GPU con menos huecos escribe menos tokens por segundo en total.
