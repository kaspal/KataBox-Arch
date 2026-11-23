## 1

Recibes 20 000 peticiones por segundo y el proveedor acepta 200. Casi todas las respuestas tienen que salir de otro sitio que no sea el proveedor. Y cuando el proveedor deja de responder, o tu copia de los datos desaparece, ¿qué recibe el usuario?

## 2

Una previsión que caducó hace un minuto sigue siendo mucho mejor que un error. Deja que la caché guarde las copias caducadas un tiempo y las sirva cuando falle la llamada al proveedor, y haz que las peticiones de una misma ciudad que falta esperen a una sola llamada en vez de hacer cada una la suya. Una caché que se reinicia vacía manda todas las peticiones al proveedor a la vez, así que ten una segunda copia que pueda tomar el relevo y volver a llenarla.

## 3

Pon un AWS ALB delante de 8 Go App Servers con un tiempo límite de llamada de 200 ms, para que una petición que espera a un proveedor caído se rinda pronto. Lee de una Redis Cache (2 GB, TTL de 600 s) con «Servir caducado ante error durante» en 3600 s, la protección de estampida activada y un tiempo de failover de 5 s. Conéctala con Replicar a una segunda Redis Cache con los mismos ajustes.
