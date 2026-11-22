# Solución comentada

## Adónde va la carga

Con 6000 mensajes/s, unos 5400 son ediciones y unos 600 son instantáneas. Las instantáneas son fáciles: menos de 10 MB/s hacia PostgreSQL.

Las ediciones son las que dan forma al diseño. El servidor de sincronización al que llega cada edición la publica, y todos los servidores de sincronización suscritos al bus la reciben. Así que cada tarjeta de sincronización trabaja con todas las ediciones del sistema, no solo con las de sus editores.
La factura no sigue a las ediciones. 5400 ediciones por segundo durante un mes son unos 14 200 millones de mensajes, pero el Pub/Sub Redis es propio: un nodo de 36 $ al mes, sin coste por mensaje. La mayor parte de la factura son los servidores de sincronización y PostgreSQL.

## El diseño de referencia

Un Network Load Balancer, que admite WebSockets, reparte a los editores entre dos tarjetas de sincronización, Sync A y Sync B, cada una con 3 instancias Go. Las dos publican las ediciones en un Pub/Sub Redis, el «bus de ediciones», y las dos consumen de él, así que una edición escrita en Sync A llega a los editores de Sync B. Las instantáneas van a una PostgreSQL primaria, con una segunda PostgreSQL en un enlace Replicate como copia de reserva.

Cuesta unos 1181 $ al mes, 36 $ de ellos por el bus. En una jornada normal, cada tarjeta de sincronización está ocupada en torno al 28 % y el p95 ronda los 29 ms.

«500 editores en un mismo documento» triplica las ediciones y hace que unos pocos documentos sean mucho más populares que el resto (Zipf 2,0). Eso no crea un servidor caliente, porque el bus entrega cada edición a todos los servidores de sincronización, sea cual sea su documento. Triplicar las ediciones casi triplica el trabajo de cada tarjeta, y a las dos les sobra margen: el p95 se queda cerca de 29 ms. El coste mensual que se ve durante esta prueba se queda en unos 1181 $, porque el bus no cobra por mensaje.

«Se cae un servidor de sincronización» tumba Sync A en el segundo 20 y admite un 3 % de errores, un 97 % de disponibilidad y un p95 de 2 segundos. El balanceador necesita dos comprobaciones de salud fallidas, separadas 2 segundos, así que durante 2 a 4 segundos sigue mandando editores a la tarjeta caída. Falla cerca del 1,7 % de los mensajes y el p95 sube a alrededor de 1 segundo. Después, Sync B lo lleva todo: cada tarjeta está dimensionada para hacerlo sola.

## Por qué falla el primer intento

Un primer intento habitual tiene el mismo balanceador, el mismo bus y la misma PostgreSQL, sin la réplica, y una sola tarjeta de sincronización de 3 instancias. Cuesta unos 621 $ al mes y supera la jornada normal, con la tarjeta ocupada en torno al 38 % y un p95 cercano a 27 ms.

En «500 editores en un mismo documento», esa tarjeta atiende cada edición publicada y cada edición entregada al triple de ritmo. Sus huecos se llenan, las ediciones hacen cola y falla cerca del 3,9 % de los mensajes. En «Se cae un servidor de sincronización» no hay una segunda tarjeta que acoja a los editores: desde el segundo 20 nadie responde, y falla alrededor del 67 % de los mensajes.

## Lo que el simulador deja fuera

Un editor real fusiona las ediciones simultáneas sobre el mismo punto con transformaciones operacionales o CRDT, aplica las ediciones de un documento en el mismo orden en todas partes y muestra quién está presente y dónde tiene el cursor. Nada de eso se simula. Este problema mide la entrega, el coste del reparto a todos los servidores y la conmutación ante fallos. A mayor escala, además, asignarías cada documento a unos pocos servidores de sincronización en lugar de enviar cada edición a todos.

## Para probar

- Añade una tercera tarjeta de sincronización y ejecuta la jornada normal. Cada tarjeta sigue recibiendo todas las ediciones: ¿cuánto baja la ocupación de cada una?
- Baja cada tarjeta de sincronización a 2 instancias y ejecuta «Se cae un servidor de sincronización».
- Junta Sync A y Sync B en una sola tarjeta de 6 instancias. Compara el coste y piensa qué pasa cuando esa tarjeta se detiene.
