# Solución comentada

## Adónde va la carga

Todas las cifras están en tiempo simulado: la ventana real de 6 horas aquí es de 600 s, 36 veces más corta. El trabajo son 100 tareas de 300 segundos-ejecutor, 30 000 segundos-ejecutor en total. Con al menos 100 ejecutores, todas las tareas empiezan a la vez, y el trabajo dura lo que su tarea más lenta: 300 s por el ruido de esa tarea (hasta cerca de 1,5), así que unos 450 s. Eso deja unos 150 s de margen, media tarea.

Una interrupción cuesta tiempo de dos formas. Primero, tira trabajo: una tarea parada a los 240 s que vuelve a empezar desde cero termina como pronto en 240 + 300 = 540 s, y más tarde con el ruido; con checkpoints conserva su última marca del 10 % y solo rehace el resto, unos 60 a 90 s. Segundo, se lleva ejecutores: después de perder la mitad, una tarea parada solo puede volver a empezar enseguida si queda un ejecutor libre. Si no, espera a que termine otra tarea (hacia los 300 a 450 s) y luego necesita su propia ejecución encima. Ningún checkpoint salva a una tarea que no puede empezar hasta los 400 s y aún tiene 240 s por delante.

Así que un clúster spot necesita las dos cosas: checkpoints, para que una interrupción tardía cueste poco trabajo, y ejecutores suficientes para que la mitad que sobrevive todavía sostenga las 100 tareas, para que una interrupción temprana no obligue a esperar. La mitad de sobra es barata: un nodo spot cuesta 49 $ al mes, el 35 % de uno bajo demanda, así que 200 ejecutores spot cuestan menos que 100 bajo demanda.

## El diseño de referencia

Una tarjeta de Cómputo por lotes Apache Spark en Amazon EMR con 50 instancias, 4 ejecutores por nodo (200 ejecutores), "Proporción spot" al 100 % y "Checkpoints" activado, conectada con Query a una tarjeta de Almacenamiento de objetos Amazon S3. Cuesta unos 2452 $ al mes: 2450 $ por los 50 nodos spot (49 $ cada uno) y 2 $ por el Almacenamiento de objetos. En una noche tranquila solo está ocupada en torno al 21 %: la mitad del clúster es una reserva para la noche en que la nube se la lleva.

"Una noche tranquila" termina en unos 455 s. "Te quitan la mitad de los ejecutores spot al principio de la noche" se lleva 100 ejecutores a los 60 s, y se paran 50 tareas en curso. Los 100 ejecutores que quedan sostienen las 50 tareas que siguieron y las 50 que vuelven a empezar enseguida desde su checkpoint, con el 80 al 90 % de su trabajo aún por hacer; el trabajo termina hacia los 460 s. "Te quitan la mitad de los ejecutores spot al final de la noche" para 50 tareas a los 240 s, cuando la mayoría ha pasado su checkpoint del 60 % al 80 %; vuelven a empezar en ejecutores libres con uno o dos minutos de trabajo pendiente, y el trabajo termina hacia los 420 s (una tarea parada vuelve a sortear su ruido, así que la tarea más lenta de la noche puede incluso acabar antes).

Salir del spot también es una opción honesta, y más cara: 25 nodos bajo demanda ("Proporción spot" al 0 %, 100 ejecutores, sin checkpoints) no se ven afectados por ninguna de las dos pruebas ocultas y las pasan en unos 450 s y 420 s, por unos 3502 $ al mes, un 43 % más que la referencia. Medio clúster de nodos spot de sobra es un seguro más barato que un clúster que nadie te puede quitar.

Para el reto de coste no hay objetivo: la prueba más barata que encontró la búsqueda, 40 nodos spot con checkpoints, cuesta el 80 % de la referencia (1962 $), que no queda por debajo de la línea del 80 %, y pasa la interrupción temprana con muy poco margen.

## Por qué falla el primer intento

Un primer intento habitual son 28 nodos todos en spot ("Proporción spot" al 100 %, 112 ejecutores, suficientes para que todas las tareas empiecen a la vez) sin checkpoints, por unos 1374 $ al mes. Pasa "Una noche tranquila" en los mismos 455 s que la referencia: en una noche tranquila más nodos no ayudan, porque 100 tareas no pueden usar más de 100 ejecutores.

"Te quitan la mitad de los ejecutores spot al final de la noche" lo rompe. A los 240 s se paran 50 tareas, la mayoría casi terminadas, y vuelven a empezar desde cero en los ejecutores libres. Cada una necesita otros 300 s completos o más, y el trabajo termina hacia los 730 s, frente a un plazo de 600 s. Los ejecutores están; el trabajo se tiró.

"Te quitan la mitad de los ejecutores spot al principio de la noche" también lo rompe. A los 60 s solo quedan 56 ejecutores, y 50 de ellos siguen con las tareas que no se pararon. Seis tareas paradas vuelven a empezar enseguida; las otras 44 esperan a que termine una tarea en curso, hacia los 300 s o más tarde, y luego necesitan una ejecución entera desde cero. El trabajo no ha terminado cuando la ejecución acaba a los 700 s.

Los dos arreglos no son intercambiables. Los mismos 28 nodos con checkpoints pasan la interrupción tardía (unos 445 s) pero siguen fallando la temprana (unos 670 s): las tareas conservan su progreso, pero esperan a un ejecutor. 50 nodos sin checkpoints pasan la interrupción temprana (unos 500 s) pero fallan la tardía (unos 695 s): cada tarea parada empieza enseguida, pero desde cero. La capacidad barata e interrumpible necesita las dos cosas: checkpoints para conservar el trabajo y ejecutores de sobra para conservar el paralelismo.

## Lo que el simulador deja fuera

El tiempo está reducido 36 veces, como dice el enunciado; lo que importa son las proporciones entre el tiempo de una tarea, el momento de la interrupción y el plazo. La capacidad spot cuesta aquí un 35 % fijo del precio bajo demanda; los precios spot reales cambian con la hora, la zona y el tipo de instancia, y son las pruebas las que deciden cuánta capacidad se lleva la nube y cuándo. Las recuperaciones reales llegan con un aviso corto, se llevan nodos enteros y no una proporción exacta, y a menudo traen nodos de reemplazo tras un tiempo de arranque; nada de eso se simula. Escribir un checkpoint aquí es gratis; uno real cuesta tiempo y almacenamiento. El modelo se ejecuta solo como tiempo de ejecutor: cargarlo, la memoria por ejecutor y la inferencia en GPU no se simulan, y la calidad de las puntuaciones no se mide en absoluto. El coste mensual supone que el clúster funciona todo el mes; un clúster nocturno real se puede apagar durante el día.

## Para probar

- Desactiva los "Checkpoints" en la referencia y ejecuta las pruebas ocultas. Compara la interrupción tardía con 240 s + 300 s.
- Baja la referencia a 28 nodos con checkpoints y mira la interrupción temprana: cuenta los ejecutores que quedan frente a las 100 tareas.
- Sube "Ejecutores por nodo" a 8 con 25 nodos. Pasa, pero cuesta lo mismo que 50 nodos: más ejecutores por nodo se cobran como una máquina más grande. En un m5.xlarge real con 4 vCPU, dos ejecutores por núcleo además harían cada tarea unas dos veces más lenta, y eso el simulador no lo modela.
- Pon la "Proporción spot" al 50 % en la referencia: cada prueba se lleva ahora una cuarta parte de los ejecutores, y la factura sube a unos 4727 $, por encima del presupuesto.
