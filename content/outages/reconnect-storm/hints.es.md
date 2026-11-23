## 1

Solo cayó una pasarela, pero casi todos los intentos de conexión fallaron durante el resto del partido. ¿Cuándo intentaron reconectarse los clientes de la pasarela caída, y adónde podían ir?

## 2

Fallaron dos cosas a la vez. Sin jitter de reconexión, los 100.000 clientes llegaron en el mismo instante y, una vez rechazados, volvían juntos cada segundo. Y las supervivientes estaban llenas al 91 %: aunque llegaran repartidos, solo había sitio para 30.000. Arreglar solo una no basta: en el simulador, con jitter pero sin margen se sigue rechazando la mayoría de los intentos, y con margen pero sin jitter siguen pasando unos quince segundos de tormenta.

## 3

En el Client, pon un jitter de reconexión de 10 s. En cada WebSocket Gateway, sube el límite de conexiones a 135.000, para que tres pasarelas puedan sostener a los 400.000 aficionados. No hace falta cambiar nada más.
