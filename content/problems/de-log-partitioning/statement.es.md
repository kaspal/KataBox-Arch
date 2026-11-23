# Buscar en un año de logs

## El producto

Cuando algo se rompe, tus ingenieros buscan en los logs de la aplicación. Los logs llegan a una tabla Apache Iceberg en almacenamiento de objetos, y la tabla guarda un año entero: unos 100 MB de columnas comprimidas al día, 36,5 GB en total. Los ingenieros la consultan con Google BigQuery, que lee la tabla donde está y cobra bajo demanda: 6,25 $ por cada TB que escanea. Casi todas las búsquedas preguntan por las últimas 24 horas. De vez en cuando alguien pregunta por el año entero.

## Lo que debes cumplir

| Objetivo | Valor |
| --- | --- |
| Tráfico pico | 2 consultas/s |
| Latencia p95 | 2 s o menos |
| Disponibilidad | 99,9 % o más |
| Errores | 1 % o menos |
| Presupuesto | 35 000 $ al mes |

El presupuesto se comprueba con el coste de «Un día de trabajo normal», como si su pico durara todo el mes.

## El tráfico

- **Buscar en los logs de las últimas 24 horas** (99 %). El Cliente de los ingenieros llama a tu Servidor de aplicaciones, que manda la consulta a un Almacén de datos. El almacén lee la Tabla lakehouse a la que está conectado con una conexión de Consulta. La consulta filtra por la marca de tiempo del log, así que necesita un día de los 365 de la tabla: 0,1 GB.
- **Consultar el año entero de logs** (1 %). El mismo camino. Necesita los 36,5 GB, se organice como se organice la tabla.

El tiempo de una consulta en el almacén es su Sobrecarga por consulta (300 ms) + los datos que escanea ÷ su Velocidad de escaneo por consulta (1 GB/s) + 20 ms por cada archivo que abre, 8 archivos a la vez. Mientras se ejecuta, una consulta ocupa uno de los huecos del almacén (Instancias × Concurrencia por instancia); las demás esperan en cola. La página de búsqueda espera hasta 60 s, así que pon el Tiempo límite del Cliente en 60 000 ms. Pasado ese tiempo, la búsqueda falla.

## Las pruebas

Hay una prueba visible. Puedes ejecutarla tantas veces como quieras:

- **Un día de trabajo normal**: el tráfico descrito arriba, en su pico, durante 5 minutos.

Hay dos pruebas ocultas más, de las que solo ves el nombre: **Una auditoría lanza quince veces las consultas del año entero de siempre** y **Un incidente: todo el mundo busca en el último día a la vez**. Durante la auditoría, las consultas del año entero son mucho más del 5 % del tráfico y tardan por sí solas más de medio minuto, así que esa prueba admite un p95 de 45 s. Tu diseño tiene que superar las tres.
