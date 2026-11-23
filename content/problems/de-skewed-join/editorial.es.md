# Solución comentada

## Adónde va la carga

Todas las cifras están en tiempo simulado: la ventana real de 4 horas son aquí 480 s, 30 veces más corta. El trabajo son 2700 segundos-ejecutor, así que 24 ejecutores podrían hacerlo en unos 2700 ÷ 24 ≈ 113 s, más la sobrecarga por tarea (2000 tareas × 0,2 s ÷ 24 ≈ 17 s). Esa es la duración del trabajo solo si el trabajo se reparte por igual, y aquí no. El tamaño de las tareas sigue las ventas de los vendedores (un sesgo Zipf de 1,0 sobre 2000 vendedores): la tienda propia del marketplace son 2700 ÷ 8,18 ≈ 330 segundos-ejecutor en una sola tarea, el 12 % de todo el trabajo, y los siguientes vendedores son 165, 110 y 83 s.

Una tarea corre en un solo ejecutor, así que el trabajo no puede terminar antes que su tarea más grande: unos 330 s por el ruido de esa tarea, más lo que esperó en la cola a su primer ejecutor. Los otros 23 ejecutores terminan todo lo demás en unos dos minutos y luego se quedan ociosos. En una noche tranquila eso todavía cabe en 480 s, pero con poco margen, y la tarea caliente está expuesta durante casi toda la noche: cualquier interrupción que la pare cuesta toda su duración, o casi toda, otra vez.

El "Salting (dividir tareas calientes)" cambia la forma del trabajo. Con salting en 16, cada una de las 20 tareas más grandes (el 1 % superior) se divide en 16 piezas iguales: la tarea de la tienda se convierte en 16 piezas de unos 21 s, y ninguna tarea del trabajo pasa de unos 21 s. La duración del trabajo queda cerca del trabajo total dividido entre los ejecutores, y una interrupción solo cuesta las piezas que estaban en marcha.

## El diseño de referencia

Una tarjeta de Cómputo por lotes Apache Spark en Amazon EMR con 6 instancias y 4 ejecutores por nodo (24 ejecutores), "Salting (dividir tareas calientes)" en 16 y "Checkpoints" activados, conectada con Query a una tarjeta de Almacenamiento de objetos Amazon S3 (los pedidos) y con Query a una tarjeta de Almacén de datos Snowflake (el informe). Cuesta unos 1329 $ al mes: 840 $ por los 6 nodos, 3 $ por el Almacenamiento de objetos y 486 $ por Snowflake, que solo cobra mientras está despierto para recibir las escrituras.

"Una noche normal" termina en unos 175 s, con cinco minutos de margen. "Te quitan todos los ejecutores spot durante dos horas, al principio de la noche" para todas las tareas en marcha a los 60 s y deja el clúster vacío hasta los 300 s; las tareas con salting son pequeñas, así que el trabajo sigue donde estaba y termina hacia los 415 s. "Te quitan todos los ejecutores spot durante 15 minutos, al final de la noche" llega a los 300 s, mucho después de que el trabajo terminara, y no cambia nada.

Los "Checkpoints" casi no cambian nada en la referencia (sin ellos termina la prueba temprana hacia los 420 s): una pieza de 21 s parada pierde como mucho 21 s. Son un seguro barato, y son lo que más necesita un trabajo sin salting, como muestra la siguiente sección.

Para el reto de coste no hay objetivo: la prueba más barata que encontró la búsqueda cuesta el 95 % de la referencia, que no baja de la línea del 80 %.

## Por qué falla el primer intento

Un primer intento habitual ve una tarea que tarda mucho y recurre a los dos arreglos de siempre: "Ejecución especulativa" activada y nodos de sobra. Con 6 nodos y especulación cuesta unos 1550 $ al mes (Snowflake se queda despierto más tiempo, mientras termina la tarea larga) y pasa "Una noche normal" en unos 415 s: la tarea caliente marca sola el tiempo, y el clúster está ocupado un 28 % de la prueba.

"Te quitan todos los ejecutores spot durante 15 minutos, al final de la noche" lo rompe. A los 300 s la tarea caliente ha hecho casi todos sus 330 s, y vuelve a empezar desde cero cuando los ejecutores regresan a los 330 s. Necesita otra ejecución entera y termina hacia los 625 s, con un plazo de 480 s.

"Te quitan todos los ejecutores spot durante dos horas, al principio de la noche" también lo rompe, y aquí los checkpoints no lo salvan. A los 60 s la tarea caliente ha hecho menos de una quinta parte de su trabajo. Cuando los ejecutores vuelven a los 300 s todavía le quedan unos 300 s, así que termina hacia los 535 s haga lo que haga el resto del clúster.

Ninguno de los arreglos habituales toca la causa:

- **La especulación** copia una tarea que tarda mucho más que la típica, pero la copia tiene los mismos 330 s de trabajo. Solo ayuda con una ejecución lenta por azar: sin ella el primer intento termina la noche tranquila hacia los 435 s en lugar de 415 s, y falla las mismas pruebas ocultas.
- **Más nodos** terminan antes las tareas pequeñas, pero no pueden dividir la grande. Con 12 nodos (y checkpoints) la noche tranquila termina hacia los 405 s, la interrupción temprana sigue hacia los 565 s, y la factura sube 840 $ al mes.
- **Los checkpoints** sin salting pasan la interrupción tardía (unos 465 s: la tarea caliente conserva su marca del 70 %) pero siguen fallando la temprana (unos 535 s), porque la tarea tenía poco progreso que conservar.

Solo el salting elimina la tarea gigante, y con ella las horas que el trabajo pasa esperando a un solo ejecutor.

## Lo que el simulador deja fuera

El tiempo está reducido 30 veces, como dice el enunciado; lo que importa son las proporciones entre el tamaño de las tareas, las interrupciones y el plazo. El sesgo del trabajo es fijo: todas las pruebas usan los mismos datos, así que las pruebas cambian cuándo la nube recupera capacidad, no lo caliente que está la clave. Aquí el salting es un ajuste; en un join real significa añadir una sal aleatoria a la clave caliente en el lado grande, copiar las filas correspondientes del lado pequeño una vez por cada valor de sal y volver a combinar los resultados parciales, lo que cuesta shuffle y memoria extra que el simulador no cobra (el tráfico de shuffle y los volcados de memoria no se simulan). El simulador divide exactamente el 1 % de tareas más grandes; un trabajo real tiene que encontrar antes sus claves calientes, con las estadísticas de la tabla o con la ejecución adaptativa de consultas. La prop "Proporción spot" no cambia nada en el simulador: un nodo cuesta el mismo precio didáctico diga lo que diga, y las pruebas deciden cuánta capacidad recupera la nube. Las recuperaciones reales llegan con un breve aviso y a menudo traen nodos de reemplazo tras un tiempo de arranque, y escribir un checkpoint aquí es gratis. El coste mensual supone que el clúster funciona todo el mes; un clúster nocturno real puede apagarse de día. No se juzga si las cifras del informe son correctas.

## Para probar

- Pon el "Salting (dividir tareas calientes)" en 1 en la referencia y ejecuta las pruebas ocultas: la duración del trabajo salta de unos 175 s a la duración de la tarea caliente.
- Prueba el salting en 4 en lugar de 16: la tarea caliente se convierte en 4 piezas de unos 80 s. Compara su tiempo en la interrupción temprana con el de la referencia.
- Duplica el primer intento a 12 nodos y compara el tiempo de la noche tranquila con el de 6 nodos: apenas se mueve.
