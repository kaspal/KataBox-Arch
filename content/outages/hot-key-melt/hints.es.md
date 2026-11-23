## 1

Durante el pico, la tarjeta de Cassandra solo está ocupada de media cerca de la mitad del tiempo y, aun así, se descartan peticiones. ¿Adónde va de verdad la carga?

## 2

Casi la mitad de las cargas de página piden un solo producto, y una clave siempre cae en la misma partición. Esa partición está llena mientras las otras tres están ociosas. Más particiones no ayudan: la clave caliente sigue viviendo en una de ellas. El producto caliente tiene que servirse desde algún sitio que no sea una partición.

## 3

Pon una caché Redis (4 GB, TTL de 300 s) delante de Cassandra: conecta los App Servers a la caché con una arista de consulta y mantén su arista a Cassandra para los fallos de caché. No hace falta cambiar nada más.
