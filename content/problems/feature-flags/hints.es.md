## 1

Casi todas las peticiones hacen la misma pregunta pequeña sobre los mismos pocos miles de flags, y la respuesta rara vez cambia. ¿De verdad cada una tiene que llegar a la base de datos?

## 2

Pon una caché delante de la base de datos (cache-aside) y deja que las entradas caduquen tras un tiempo fijo. Ese tiempo es el compromiso: cuanto más vive una entrada, menos lecturas llegan a la base de datos, pero más tarda en verse un cambio. Una caché que todavía guarda los flags también sigue respondiendo mientras la base de datos no está.

## 3

Añade una Cache Redis y ponle un TTL de unos pocos segundos, unos 3, en lugar de los 300 por defecto: así un cambio se ve en segundos y a la base de datos solo le llega un goteo de fallos de caché. Mantén los App Servers Go y una PostgreSQL principal con una réplica unida por una arista Replicate. Sin la caché, cada miembro de PostgreSQL atiende unas 20 000 consultas por segundo, muy lejos de 60 000.
