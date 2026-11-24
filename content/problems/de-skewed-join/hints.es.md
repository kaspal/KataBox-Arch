## 1

Busca el camino crítico. El trabajo termina cuando termina su última tarea, y la tarea más grande son unos 330 segundos-ejecutor, quizá hasta 1,5 veces eso. En una noche tranquila eso ya se come buena parte de los 480 s. Ahora imagina que la nube para esa tarea y que los ejecutores no vuelven hasta los 300 s. ¿Cuánto le queda por ejecutar, y cuándo puede terminar?

## 2

Añadir nodos o activar la "Ejecución especulativa" no hace más pequeña la tarea más grande: una tarea corre en un solo ejecutor, y una copia de una tarea enorme es igual de enorme. Los "Checkpoints" salvan parte del trabajo de una tarea parada, pero la tarea caliente sigue necesitando casi todos sus 330 s cuando vuelven los ejecutores. Lo que la encoge es el "Salting (dividir tareas calientes)": divide las tareas más grandes en suficientes piezas para que cada una sean unas decenas de segundos, y la duración del trabajo pasa a ser el trabajo total dividido entre los ejecutores. A partir de ahí más nodos sí acortan el trabajo, y los nodos spot son baratos: unos 61 $ al mes cada uno con la "Proporción spot" al 100 %.

## 3

Tarjeta de Cómputo por lotes Apache Spark en Amazon EMR con 10 instancias, 4 ejecutores por nodo (40 ejecutores), "Proporción spot" al 100 %, "Salting (dividir tareas calientes)" en 16 y "Checkpoints" activados, conectada con Query a una tarjeta de Almacenamiento de objetos Amazon S3 y con Query a una tarjeta de Almacén de datos Snowflake.
