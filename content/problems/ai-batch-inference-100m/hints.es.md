## 1

Calcula la línea de tiempo de una tarea. Una tarea tarda unos 300 s, y el plazo es de 600 s. Si una tarea se para a los 240 s de empezar y tiene que volver a empezar desde cero, ¿cuándo termina como pronto? Ahora piensa en una tarea parada a los 60 s, cuando la mitad de los ejecutores ya no está y todos los que quedan están ocupados.

## 2

Fallan dos cosas, y necesitan dos arreglos distintos. Una interrupción tardía tira casi una tarea entera de trabajo: los "Checkpoints" de la tarjeta de Cómputo por lotes la conservan, así que una tarea parada solo rehace su último 10 %. Una interrupción temprana se lleva la mitad de los ejecutores para el resto de la noche: si lo que queda es menos que las 100 tareas, algunas tareas esperan a que termine otra y luego necesitan una ejecución entera para ellas. Dimensiona el clúster para que la mitad todavía sostenga todas las tareas. Los nodos spot de sobra son baratos: con la "Proporción spot" al 100 %, un nodo cuesta 49 $ al mes, más o menos un tercio de uno bajo demanda.

## 3

Tarjeta de Cómputo por lotes Apache Spark en Amazon EMR con 50 instancias, 4 ejecutores por nodo (200 ejecutores), "Proporción spot" al 100 % y "Checkpoints" activado, conectada con Query a una tarjeta de Almacenamiento de objetos Amazon S3.
