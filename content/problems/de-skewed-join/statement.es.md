# Un join con una clave gigante

## El producto

Un marketplace genera cada noche el informe de ventas diario de sus vendedores. Un trabajo de Cómputo por lotes (Apache Spark en Amazon EMR) une los pedidos del día, guardados en Almacenamiento de objetos, con la tabla de vendedores, los agrupa por vendedor y escribe el resultado en un Almacén de datos (Snowflake). Analistas y vendedores abren el informe a primera hora de la mañana, así que tiene que estar listo para entonces.

Los datos están sesgados. La mayoría de los 2000 vendedores vende poco, unos pocos venden mucho, y la tienda propia del marketplace vende tanto que tiene alrededor del 12 % de todos los pedidos. El join reparte el trabajo por vendedor, una tarea por partición de vendedores, así que esa clave hace que una tarea sea muchísimo más grande que todas las demás.

El clúster usa instancias spot, porque son mucho más baratas. La nube puede recuperarlas en cualquier momento, y todas las tareas que corren en ellas se paran.

**Tiempo a escala.** El trabajo real tiene una ventana de 4 horas. El simulador ejecuta minutos, así que el tiempo se reduce 30 veces: la ventana son 480 segundos simulados, y 1 segundo simulado equivale a 30 segundos reales. El join entero son 2700 segundos-ejecutor de trabajo en el simulador (unas 22 horas-ejecutor reales). Todas las cifras de abajo están en tiempo simulado.

## Lo que debes cumplir

| Objetivo | Meta |
| --- | --- |
| Trabajo | 2700 segundos-ejecutor en 2000 tareas sesgadas, que empiezan en 0 s |
| Plazo | el trabajo entero termina en 480 s desde que empieza |
| Errores | 0,1 % o menos |
| Presupuesto | 1650 $ al mes |

El coste mensual es lo que costarían las tarjetas si funcionaran todo el mes, la misma regla que en todos los demás problemas (un nodo de Cómputo por lotes cuesta 140 $ al mes bajo demanda, y la parte de sus ejecutores que marca la "Proporción spot" corre en capacidad spot al 35 % de ese precio, así que un nodo al 100 % cuesta 49 $; Snowflake solo cobra mientras está despierto). Los precios son los precios didácticos del registro.

## El tráfico

- **Unir los pedidos de esta noche con los vendedores** (un trabajo). Empieza en 0 s y se divide en 2000 tareas, una por partición de vendedores. El tamaño de las tareas sigue las ventas de los vendedores: la tarea más grande, la tienda propia del marketplace, son unos 330 segundos-ejecutor de trabajo; la segunda, más o menos la mitad; la mayoría tarda menos de un segundo. Cada tarea espera a un ejecutor libre, lo ocupa durante su trabajo (cada ejecución varía un poco, hasta unas 1,5 veces en el peor caso, más la sobrecarga por tarea), y luego lee del Almacenamiento de objetos y escribe en el Almacén de datos. El trabajo termina cuando termina su última tarea.

Un nodo de Cómputo por lotes es una máquina de la clase m5.xlarge con 4 vCPU, así que deja "Ejecutores por nodo" en 4: lo que dimensionas es el número de nodos. Más ejecutores por nodo se cobran como una máquina más grande, en proporción. Tareas en paralelo = nodos × ejecutores por nodo. Una tarea corre en un solo ejecutor: más ejecutores nunca acortan una tarea concreta.

La tarjeta de Cómputo por lotes tiene tres ajustes para este tipo de trabajo:

- **Salting (dividir tareas calientes)**: el join añade una "sal" aleatoria a las claves más calientes, así que cada una del 1 % de tareas más grandes se divide en ese número de piezas iguales que corren en ejecutores distintos.
- **Ejecución especulativa**: cerca del final del trabajo (con el 95 % de las tareas hechas), una tarea que lleva mucho más tiempo que la típica recibe una segunda copia, y gana la primera copia que termina.
- **Checkpoints**: una tarea parada por una interrupción spot vuelve a empezar desde su último checkpoint (uno cada 10 % de la tarea) en lugar de desde cero.

Cuando la nube recupera capacidad spot, esa parte de los ejecutores spot de la tarjeta desaparece hasta que la capacidad vuelve, y la misma parte de sus tareas en marcha se para y vuelve a empezar. Los ejecutores bajo demanda nunca se los lleva: con la "Proporción spot" al 100 % todo el clúster está expuesto, y al 0 %, nada.

## Las pruebas

Una prueba es visible. Puedes ejecutarla tantas veces como quieras:

- **Una noche normal**: el trabajo corre sin interrupciones.

Hay dos pruebas más ocultas, de las que solo ves el nombre: **Te quitan todos los ejecutores spot durante dos horas, al principio de la noche** y **Te quitan todos los ejecutores spot durante 15 minutos, al final de la noche**. Tu diseño tiene que pasar las tres.
