## 1

Calcula cuánto tarda una consulta por sí sola, sin nadie más en el almacén de datos. ¿Cuántos archivos abre para un día normal, y para un día de rebajas? Si eso solo ya roza el límite, ¿pueden ayudar más instancias del almacén?

## 2

El tiempo de una consulta es su sobrecarga, más los datos que lee a la velocidad de escaneo, más unos 20 ms por cada 8 archivos que abre. Con archivos pequeños manda la última parte: 250 MB en archivos de 500 KB son 512 archivos. Júntalos en archivos grandes en segundo plano, para que cada consulta abra unos pocos en vez de cientos. Después comprueba que el almacén tiene huecos suficientes para cuatro veces las consultas, y velocidad de escaneo total suficiente (sus instancias se la reparten) para las lecturas más grandes de un día de rebajas.

## 3

Mantén la Tabla lakehouse (Delta Lake) particionada por día, pon la Compactación en Cada hora y el tamaño de archivo objetivo en 256 MB. Léela desde un Almacén de datos (Databricks SQL) con 4 instancias, una concurrencia de 8 y una velocidad de escaneo de 5 GB/s, con la caché de resultados activada. Alimenta la tabla por un stream de Apache Kafka hacia 2 instancias de Apache Flink, detrás de un AWS ALB y 2 servidores de aplicaciones de Go, y pon el tiempo límite del Cliente en 60 000 ms.
