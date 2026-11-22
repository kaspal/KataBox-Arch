## 1

¿Adónde va el 95 % del tráfico, y de verdad cada clic necesita llegar a la base de datos?

## 2

Pon una caché delante de la base de datos (cache-aside) para que los enlaces populares casi nunca la toquen. Después pregúntate qué les pasa a las creaciones y a las lecturas cuando tu única base de datos se detiene.

## 3

Añade una Cache Redis de unos pocos GB (en 8 GB caben unos 8 millones de entradas de 1 KB) y una réplica de PostgreSQL unida a la principal con una arista Replicate y un tiempo de failover corto. Deja a la capa de aplicación mucho margen por encima del pico, no solo lo justo para un día normal.
