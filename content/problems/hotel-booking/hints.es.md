## 1

Casi todas las peticiones son búsquedas, y las mismas pocas búsquedas vuelven una y otra vez. Unas pocas reservas compiten por las mismas habitaciones en el mismo momento. Son dos preocupaciones distintas: dónde caen las búsquedas populares y quién se queda la última habitación.

## 2

Un motor de búsqueda divide sus datos en particiones (shards), y cada búsqueda va a la partición que la contiene. Añadir particiones reparte muchas búsquedas distintas, pero no puede dividir una búsqueda popular: toda ella sigue cayendo en una partición. Una caché delante del motor de búsqueda responde a las búsquedas populares antes de que lleguen. Para las reservas, leer y luego escribir deja que dos viajeros se queden la misma habitación; una escritura que falla si la noche ya está reservada deja ganar solo a uno.

## 3

Pon un AWS ALB delante de 4 Go App Servers. Añade una Redis Cache (4 GB, TTL 300 s) delante de una tarjeta Elasticsearch con 4 shards. En la tarjeta PostgreSQL, activa «Escrituras condicionales (comprobar y escribir en un paso)», para que la reserva sea una sola sentencia que falla si esa noche de la habitación ya está ocupada.
