## 1

Casi todas las peticiones solo leen un código, y se leen una y otra vez los mismos pocos códigos. Una de cada veinte cambia una cuenta, y muchas de esas tocan el mismo código en el mismo momento. Son tres preocupaciones distintas: las lecturas, la carrera por la cuenta y qué pasa cuando la única base de datos se para.

## 2

Los códigos cambian poco comparado con lo a menudo que se consultan, así que una caché delante de la base de datos le quita la mayoría de las consultas y libera antes los App Servers. La cuenta que se ve al pagar puede ir un poco por detrás; lo que tiene que ser exacto es el canje. Leer y después escribir deja que dos clientes se lleven el último uso; una escritura que comprueba la cuenta y resta uno en el mismo paso deja ganar solo a uno. Y una única primaria sin nada a lo que pasar el relevo se lleva todos los canjes cuando cae.

## 3

Pon un AWS ALB delante de 2 Go App Servers. Añade una Redis Cache (0,5 GB sobran, TTL de 60 s) para las consultas. En la tarjeta PostgreSQL, activa «Escrituras condicionales (comprobar y escribir en un paso)» y pon su Tiempo de failover en 10 s, y después conecta con una arista Replicate una segunda tarjeta PostgreSQL con los mismos ajustes.
