## 1

Dos personas escriben en el mismo documento, pero el balanceador las ha puesto en servidores de sincronización distintos. ¿Cómo llega una edición que entra por un servidor a la persona conectada al otro?

## 2

Haz que todos los servidores de sincronización publiquen sus ediciones en un único bus compartido y se suscriban a él, para que cada servidor reciba todas las ediciones, vengan de donde vengan. Después asegúrate de que ninguna tarjeta tenga todos los servidores de sincronización, y de que la que quede en pie pueda con toda la carga, incluida una reunión que escribe el triple.

## 3

Pon un Network Load Balancer delante de dos tarjetas de App Servers Go con 3 instancias cada una. Conecta las dos tarjetas para que publiquen en un Pub/Sub Redis y consuman de él, y escribe las instantáneas en PostgreSQL. Un Pub/Sub Redis propio no cobra por mensaje, así que el coste mensual son sobre todo las tarjetas de sincronización y la base de datos: mira cuánto suma cada una antes de añadir más de nada.
