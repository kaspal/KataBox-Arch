## 1

¿Adónde va el historial de un usuario de la UE en un día normal, y adónde va después de que la base de datos conmute? Una réplica es donde cae el trabajo del primario cuando este se pierde, así que la réplica debe cumplir la misma regla que el primario. Y cuando se pierde una región entera, sus GPU se van con ella: lo que queda debe llevar todos los mensajes.

## 2

Deja el primario del historial en una Región con la etiqueta `eu`, y pon su réplica en una segunda Región también con la etiqueta `eu` (por ejemplo, eu-west y eu-central), no junto a tus servidores de EE. UU. Todos los Servidores de aplicaciones, en EE. UU. o en la UE, escriben en esa única base de datos de la UE: el historial de los usuarios de EE. UU. también puede vivir en la UE. Para las GPU, cuenta tokens: 30 mensajes por segundo de unos 250 tokens son 7500 tokens por segundo o más, y una GPU con el lote lleno escribe como mucho 64 × 40 = 2560. Cuando se pierde un grupo de GPU, el otro debe llevarlo todo, así que cada grupo necesita unas 4 GPU, y cada Servidor de aplicaciones debería estar conectado a los dos grupos.

## 3

Un Cliente web (Tiempo límite 30 000 ms, 2 Reintentos, Espera entre reintentos 200 ms, Variación de la espera Completa) llama a una tarjeta DNS Route 53 (TTL 30 s) que apunta a dos regiones: us-east (Etiqueta de residencia `us`) y eu-west (etiqueta `eu`). Cada una tiene un AWS ALB, 4 Servidores de aplicaciones Go y una tarjeta vLLM en GPU con 4 Instancias y los Huecos de lote por GPU en 64. Conecta cada grupo de Servidores de aplicaciones a las dos tarjetas de vLLM. Pon un primario PostgreSQL en eu-west y una réplica PostgreSQL en una tercera región, eu-central (etiqueta `eu`), unidas con una conexión Replicar, las dos con el Tiempo de failover en 10 s. Conecta los dos grupos de Servidores de aplicaciones al primario con conexiones Consulta.
