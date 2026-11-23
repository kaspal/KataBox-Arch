# Puntuar 100 millones de elementos en una noche

## El producto

Una tienda online ordena todo su catálogo con un modelo: cada noche, cada uno de sus 100 millones de elementos recibe una puntuación de relevancia nueva, y las páginas de búsqueda y de recomendaciones de la mañana leen esas puntuaciones. Los elementos están en Almacenamiento de objetos, en 100 fragmentos de 1 millón de elementos cada uno. Un clúster de Cómputo por lotes (Apache Spark en Amazon EMR) ejecuta el trabajo: una tarea por fragmento lee el fragmento, ejecuta el modelo de puntuación en los ejecutores y escribe las puntuaciones de vuelta en Almacenamiento de objetos. El modelo es un archivo pequeño que carga cada ejecutor, así que el trabajo es tiempo de ejecutor, no llamadas a un servicio de modelos.

El clúster usa instancias spot, porque son mucho más baratas. El precio del spot es que la nube puede llevarse nodos en cualquier momento, y todas las tareas que corren en ellos se paran. Aun así, las puntuaciones tienen que estar listas por la mañana.

**Tiempo a escala.** El trabajo real tiene una ventana de 6 horas. El simulador ejecuta minutos, así que el tiempo se reduce 36 veces: la ventana es de 600 segundos simulados, y 1 segundo simulado representa 36 segundos reales. Un fragmento necesita 300 segundos-ejecutor de trabajo en el simulador (unas 3 horas reales en un ejecutor, unos 90 elementos por segundo). Todas las cifras de abajo están en tiempo simulado.

## Lo que debes cumplir

| Objetivo | Meta |
| --- | --- |
| Trabajo | 100 tareas de 300 segundos-ejecutor cada una, que empiezan en 0 s |
| Plazo | el trabajo entero termina en 600 s o menos desde que empieza |
| Errores | 0,1 % o menos |
| Presupuesto | 12 000 $ al mes |

El coste mensual es lo que costaría el clúster si funcionara todo el mes, la misma regla que en todos los demás problemas (un nodo de Cómputo por lotes cuesta 140 $ al mes, sea cual sea su Proporción spot). Los precios son los precios didácticos del registro.

## El tráfico

- **Puntuar los elementos de esta noche** (un trabajo). Empieza en 0 s y se divide en 100 tareas iguales, una por fragmento. Cada tarea espera a un ejecutor libre, lo ocupa durante su trabajo (cada ejecución varía un poco, hasta cerca de 1,5 veces en el peor caso, más la sobrecarga por tarea), y luego lee de Almacenamiento de objetos y escribe en él. El trabajo termina cuando termina su última tarea.

Un nodo de Cómputo por lotes es una máquina de clase m5.xlarge con 4 vCPU, así que deja "Ejecutores por nodo" en 4: lo que dimensionas es el número de nodos. Tareas en paralelo = nodos × ejecutores por nodo.

Cuando la nube recupera capacidad spot, una parte de los ejecutores del clúster desaparece hasta que la capacidad vuelve, y la misma parte de las tareas en curso se para y vuelve a empezar. Sin checkpoints, una tarea parada empieza otra vez desde cero. Con "Checkpoints" en la tarjeta de Cómputo por lotes, empieza otra vez desde su último checkpoint (uno cada 10 % de la tarea). En las pruebas, la parte que se lleva la nube no depende de la "Proporción spot" de la tarjeta: trata todo el clúster como spot.

## Las pruebas

Una prueba es visible. Puedes ejecutarla tantas veces como quieras:

- **Una noche tranquila**: el trabajo se ejecuta sin interrupciones.

Hay dos pruebas más ocultas, y solo ves sus nombres: **Te quitan la mitad de los ejecutores al principio de la noche** y **Te quitan la mitad de los ejecutores al final de la noche**. Tu diseño tiene que pasar las tres.
