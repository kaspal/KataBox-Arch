## 1

Imagina que tienes un solo gateway y desaparece. ¿Qué les pasa a todas las conversaciones abiertas en él, y a cada mensaje que se envíe después?

## 2

Pon varios gateways detrás de un balanceador de carga y únelos con pub/sub. Si todos los gateways se suscriben al mismo bus, cualquiera de ellos puede entregar un mensaje, sin importar a qué gateway llegó.

## 3

Pon al menos dos tarjetas de gateway detrás de un Network Load Balancer que admita WebSockets. Haz que cada tarjeta publique en un único bus Pub/Sub y consuma de él, y dales instancias suficientes para una tarde mucho más movida de lo normal. Tener más instancias dentro de una sola tarjeta no sirve de nada si se detiene la tarjeta entera.
