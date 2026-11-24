# Solución comentada

## Adónde va la carga

Los servicios hacen poco: pasan cada pregunta y esperan la respuesta. La carga está en los modelos, y en el modelo grande se cuenta en tokens por minuto.

El número de tokens varía de una pregunta a otra, y unas pocas son mucho más largas que la típica, así que las medias quedan algo por encima de las cifras del enunciado: unos 1420 tokens por pregunta fácil (entrada y salida juntas) y unos 4600 por pregunta difícil. En el pico, las preguntas fáciles envían 8 × 60 × 1420 ≈ 682 000 tokens por minuto y las difíciles 2 × 60 × 4600 ≈ 552 000. Las preguntas fáciles solo son pequeñas una a una; hay cuatro veces más, así que son más de la mitad de los tokens.

Con todas las preguntas en el modelo grande, la clave lleva unos 1 234 000 tokens por minuto, el 82 % de sus 1 500 000. Un día normal cabe. El boletín triplica las preguntas fáciles: unos 2 046 000 + 552 000 ≈ 2 600 000 tokens por minuto. La clave guarda la asignación de un minuto y la rellena a medida que pasa el minuto, así que se agota tras unos 1 500 000 / (2 600 000 − 1 500 000) de minuto, unos 80 segundos, y desde ahí rechaza unas 4 de cada 10 llamadas con un 429.

La factura sigue a los mismos tokens. El modelo grande cuesta 2 $ por millón de tokens de entrada y 10 $ por millón de tokens de salida (los precios de lista del proveedor, como los cita el registro), así que solo las preguntas fáciles cuestan unos 86 000 $ al mes en él. Un nodo GPU de vLLM cuesta 734 $ al mes (un nodo de la clase g5.xlarge), sirva lo que sirva.

## El diseño de referencia

Un Cliente web con un Tiempo límite de 60 000 ms envía las preguntas a un API Gateway Kong, que pasa las fáciles a 2 Servidores de aplicaciones Go y las difíciles a 2 Microservicios. Los Servidores de aplicaciones llaman a una tarjeta vLLM en GPU con el modelo pequeño, con 4 Instancias (16 Huecos de lote por GPU cada una, el valor por defecto) y la Velocidad de salida en 150. Los Microservicios llaman a una tarjeta API de Claude con el Límite de tokens por minuto en 1 500 000. Cuesta unos 112 826 $ al mes: 109 462 $ por los tokens de las preguntas difíciles en la API de Claude, 2936 $ por los 4 nodos GPU, 148 $ por el gateway y 140 $ cada uno por los Servidores de aplicaciones y los Microservicios.

En un día normal solo las preguntas difíciles llegan a la clave: unos 552 000 tokens por minuto, el 37 % de su límite. Una respuesta fácil ocupa un hueco de lote durante unos 1,3 s, así que 8 por segundo mantienen ocupados unos 11 de los 64 huecos: el 17 %. El p95 de la respuesta entera es de unos 18,0 s, marcado por las respuestas difíciles (1200 tokens a 100 por segundo son 12 s), no hay errores y el tiempo hasta el primer token queda por debajo de su objetivo de 2,5 s.

"El doble de preguntas difíciles" duplica el tráfico difícil durante 2 minutos. La clave lleva ahora unos 1 100 000 tokens por minuto, el 74 % de su límite. El p95 es de unos 19,8 s, no hay errores y el tiempo hasta el primer token queda por debajo de 2,5 s: el primer token de una pregunta difícil tarda los 1,1 s del proveedor más el tiempo de leer su prompt, y con más preguntas difíciles en la mezcla, más de los primeros tokens más lentos son suyos.

"Un boletín trae el triple de preguntas fáciles" dura 3 minutos. La clave ni lo nota. Las GPU reciben 24 preguntas fáciles por segundo, con unos 36 de los 64 huecos ocupados; cada respuesta se decodifica algo más despacio cuando hay más huecos ocupados, pero sigue en torno a un segundo y medio. El p95 es de unos 12,8 s (más parte del tráfico son respuestas cortas) y no hay errores.

Este problema no tiene reto de coste. Los tokens de las preguntas difíciles son el 97 % de la factura y los fija el tráfico; el diseño más barato que encontró la búsqueda y que pasa todas las pruebas cuesta unos 110 868 $, el 98 % de la referencia.

## Por qué falla el primer intento

Un primer intento habitual es el mismo gateway con los mismos servicios, con los Servidores de aplicaciones y los Microservicios llamando a una única tarjeta API de Claude. Cuesta unos 196 056 $ al mes, dentro del presupuesto, y pasa "Un día normal" con el mismo p95 de unos 18,0 s.

"El doble de preguntas difíciles" también pasa: unos 1 786 000 tokens por minuto es más de lo que permite la clave, pero a ese ritmo el minuto guardado dura unos 5 minutos, y la prueba dura 2. Una ola más larga lo rompería.

El boletín lo rompe con un 17,9 % de errores, más o menos. Hacia el segundo 77 se agota el minuto guardado de la clave, y desde ahí, segundo tras segundo, entre el 10 % y el 50 % de las llamadas se rechazan con un 429, tanto preguntas difíciles como fáciles. Los servicios no son el problema: en un día normal los Servidores de aplicaciones están ocupados en torno al 16 % y los Microservicios en torno al 21 %. El problema es el límite de tokens de la clave.

Los reintentos no lo arreglan. Con 3 Reintentos, una Espera entre reintentos de 200 ms y la Variación de la espera en Completa (espera aleatoria) en el cliente, los errores bajan a un 10,4 % más o menos, y sigue fallando: un reintento espera una fracción de segundo, y la clave necesita un minuto para rellenarse.

El enrutado de la referencia con muy pocas GPU también falla: con 2 Instancias en la tarjeta vLLM, las 24 preguntas fáciles por segundo llenan los 32 huecos, las respuestas hacen cola y se ralentizan, y el boletín acaba con un 20,7 % de errores, más o menos, y un p95 de unos 57,8 s. Con 3 Instancias pasa; la referencia mantiene 4 para tener margen.

La idea es esta: un modelo grande sale caro en dinero y en cuota, y la mayoría de las preguntas no lo necesitan. Envía cada tipo de pregunta al modelo más barato que pueda con ella. Así los tokens por minuto del modelo grande, y su factura, van al trabajo que los necesita, y una ola de preguntas fáciles cae sobre capacidad que dimensionas y pagas por mes.

## Lo que el simulador deja fuera

El simulador no juzga las respuestas. Si las respuestas del modelo pequeño a las preguntas fáciles son lo bastante buenas es la verdadera pregunta detrás de este diseño, y tienes que responderla fuera del simulador: con un conjunto de evaluación, con las valoraciones de los usuarios y con una forma de pasar una pregunta al modelo grande cuando el pequeño no está seguro. Aquí las preguntas llegan ya etiquetadas como fáciles o difíciles; un enrutador real tiene que decidirlo, con reglas o con un clasificador, y a veces se equivoca. La velocidad del modelo pequeño es la Velocidad de salida que pones, no un modelo medido. Los nodos GPU arrancan al instante y nunca fallan aquí; los reales tardan minutos en cargar un modelo y necesitan repuestos. El nivel del proveedor lo fija el problema: las tarjetas de la API de Claude muestran sus límites sin poder cambiarlos. El caché de prompts, el streaming más allá del primer y el último token y la cola propia del proveedor no se simulan. Los precios de los tokens y del nodo GPU son las cifras del registro, tomadas de los precios de lista de los proveedores.

## Para probar

- Pon la tarjeta vLLM en 2 Instancias y ejecuta el boletín. Busca el segundo en que las respuestas empiezan a hacer cola.
- Conecta los Servidores de aplicaciones a la tarjeta API de Claude además de a la tarjeta vLLM. ¿Qué parte de las preguntas fáciles gasta ahora los tokens de la clave, y qué le pasa a la factura?
- Conecta los Microservicios a la tarjeta vLLM en lugar de a la API de Claude, y mira los huecos de las GPU y el p95. El simulador te deja; si las respuestas seguirían siendo lo bastante buenas es la pregunta que no puede resolver.
