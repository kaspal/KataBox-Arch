## 1

El refresco de un gráfico ocupa un hueco del almacén mientras dura su escaneo. ¿Cuánto de la tabla lee un refresco, y en cuántos archivos? Luego imagina cinco veces más refrescos a la vez.

## 2

Particiona la tabla por día, para que un refresco lea 2 GB en vez del año entero. Luego mira los archivos: 2 GB en archivos de 2 MB son 1024 archivos que abrir, y abrirlos tarda más que leer los datos. La compactación los reescribe en unos pocos archivos grandes. En la ingesta, un día de lanzamiento envía cinco veces más lotes, así que dale al Procesador de streams huecos de sobra y al Stream particiones suficientes para alimentarlos; ese mismo margen vacía el atraso cuando el procesador vuelve.

## 3

Pon un AWS ALB delante de 6 Servidores de aplicaciones Go que publican en Apache Kafka con 96 particiones. Un Procesador de streams Apache Flink con 20 instancias (la concurrencia por defecto de 4) escribe en una Tabla lakehouse Apache Iceberg particionada por día, con compactación cada hora y un tamaño de archivo objetivo de 256 MB. Un Almacén de datos Snowflake con 5 instancias, una concurrencia de 32 y una velocidad de escaneo de 2 Gbps lee esa tabla por una arista Consulta.
