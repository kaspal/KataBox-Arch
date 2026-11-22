## 1

¿Cuánto le cuesta a una base de datos relacional guardar un valor de 1 MB, y qué deja de poder hacer mientras está ocupada con él?

## 2

Guarda el contenido de los pastes en almacenamiento de objetos, que tarda más o menos lo mismo sea cual sea el tamaño, y pon en caché las lecturas populares para que no lleguen al almacén.

## 3

Conecta los App Servers a Object Storage (S3) para el contenido y a una Cache Redis de unos pocos GB para los pastes calientes; no necesitas ninguna SQL Database. Cada creación mantiene ocupado un hilo del App Server mientras sube el contenido, así que usa unos 4 App Servers Go para dejar margen ante una ráfaga.
