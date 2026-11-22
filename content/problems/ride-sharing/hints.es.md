## 1

¿Puede una sola base de datos principal absorber decenas de miles de escrituras pequeñas por segundo, con margen de sobra cuando la ciudad se anima?

## 2

Particiona las escrituras de ubicación en un NoSQL Store, para que cada partición reciba solo una parte de los conductores. Después piensa qué hace una clave muy popular: siempre cae en la misma partición.

## 3

Usa Cassandra con suficientes particiones para que el pico, y bastante más, quepa con holgura; cada partición maneja unas 10 000 escrituras por segundo. Guarda el índice de conductores cercanos en una Cache Redis y deja a los App Servers margen por encima del pico.
