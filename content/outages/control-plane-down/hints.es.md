## 1

El grupo de AZ-b siguió pidiendo más instancias, y no arrancó ninguna. ¿Qué necesitaba el diseño para sobrevivir a la pérdida de una zona, y estaba disponible durante el evento?

## 2

Escalar y reemplazar instancias pasan por el plano de control de la nube, y estaba caído. Un diseño que aguanta la pérdida de una zona tiene que tener ya en marcha capacidad suficiente en las zonas que quedan, sin arrancar nada nuevo.

## 3

Añade una tercera zona (AZ-c) con su propio grupo de autoescalado y su App Server, y pon seis instancias en cada una de las tres zonas, con un mínimo de seis. Así dos zonas aguantan el doble del pico de la tarde si se pierde una.
