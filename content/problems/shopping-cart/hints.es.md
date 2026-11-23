## 1

En una tarde normal bastan unas pocas instancias de la aplicación. En un día grande, el grupo de autoescalado añade más. ¿Qué otra cosa crece cada vez que añade una?

## 2

Cada hueco de la aplicación que habla con PostgreSQL directamente ocupa una conexión propia, así que la base ve instancias × concurrencia conexiones, y rechaza todo lo que pase de 1700. Un pool de conexiones delante de la base mantiene un número fijo de conexiones, haya las instancias que haya, y en modo transacción presta una conexión solo lo que dura una transacción.

## 3

Pon un Pool de conexiones PgBouncer entre los Servidores de aplicaciones y PostgreSQL, con Modo en «Por transacción» (el valor por defecto) y Conexiones a la base de datos en unas 800. Mantén los Servidores de aplicaciones Spring Boot (Concurrencia por instancia 200) en un AWS Auto Scaling Group con Instancias mínimas 6, Instancias máximas 24 y Utilización objetivo del 60 %. Una caché Redis (8 GB, TTL de 600 s) responde a la mayoría de las vistas del carrito, y PostgreSQL con 2 Shards recibe las escrituras.
