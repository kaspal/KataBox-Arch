## 1

Todas las peticiones necesitan el almacén de sesiones, y no hay nada detrás. ¿Qué le pasa al sitio si esa única tarjeta deja de responder?

## 2

Haz que los App Servers sean intercambiables y repártelos en más de una tarjeta, para que perder una deje aún bastantes. Después dale al almacén de sesiones una segunda copia que pueda tomar el relevo cuando falle la primera.

## 3

Pon un Load Balancer delante de dos tarjetas de App Servers Node.js con 5 instancias cada una. Haz del almacén de sesiones una Cache Redis con espacio para 2 millones de sesiones de 2 KB, con un TTL de un día, y conecta a ella un segundo Redis con un enlace Replicate. Cuando cae el primario, la réplica pasa a primario tras el tiempo de conmutación.
