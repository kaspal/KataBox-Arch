## 1

Cada hilo es una clave. ¿Dónde vive esa clave en tu almacén de datos, y qué le pasa a ese sitio cuando todo el mundo quiere el mismo hilo a la vez?

## 2

Un almacén particionado elige la partición por la clave, así que todas las lecturas de un hilo caen en una partición. Más particiones reparten muchos hilos, pero nunca dividen un hilo. Algo que responda una clave popular desde memoria, antes del almacén, le quita esa carga a la partición. Asegúrate de que el almacén pueda seguir con los fallos de caché si alguna vez se pierde esa copia.

## 3

Pon una Redis Cache de unos 4 GB con un TTL de 300 s entre los Go App Servers y Cassandra, y conecta los App Servers a las dos. Con la caché delante, una sola partición de Cassandra basta para los fallos de caché y los comentarios. 4 instancias de Go App Servers detrás de un AWS ALB llevan 6000 peticiones por segundo con margen para un pico.
