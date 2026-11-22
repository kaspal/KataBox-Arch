# Solución comentada

## Adónde va la carga

Con 2 000 peticiones/s, unas 1 700 son lecturas y unas 300 son creaciones. El peso está en las creaciones: cada una lleva hasta 1 MB. En este simulador una SQL Database se vuelve más lenta con el tamaño de la carga, por un factor de 1 + KB / 64. Una escritura de 1 MB tarda, por tanto, unas 17 veces lo que una pequeña, alrededor de 85 ms, y retiene una de las conexiones de la base de datos todo ese tiempo. Object Storage se comporta de otra forma: unos 50 ms sea cual sea el tamaño, y muchas más peticiones en paralelo.

Las lecturas siguen una Zipf 1,0 sobre 5 millones de pastes, así que una caché delante del almacén atrapa buena parte de ellas.

## El diseño de referencia

Un Load Balancer reparte el tráfico entre 4 App Servers Go. Las lecturas consultan primero una Cache Redis de 4 GB y, si fallan, van a Object Storage (S3). Las creaciones escriben el contenido directamente en Object Storage. El diseño no tiene ninguna SQL Database.

Cuesta unos 2 579 $ al mes, y la mayor parte son peticiones a Object Storage, que en este modelo cuestan 1 $ por millón. Supera la ráfaga con un p95 cercano a 127 ms, y resiste la caché fría porque Object Storage puede atender todas las lecturas por sí solo mientras la caché se calienta.

## Por qué falla el primer intento

Un primer intento natural es un Load Balancer, 2 App Servers Go y una base de datos PostgreSQL que lo guarda todo. En un día normal pasa, con un p95 cercano a 125 ms.

«Una ráfaga de pastes grandes» envía 5 veces las creaciones habituales, unas 1 500 escrituras grandes por segundo. Cada una retiene una conexión unos 85 ms, así que las 100 conexiones se llenan. Los hilos de los App Servers esperan a la base de datos sin soltar su hueco, los App Servers también se quedan sin hilos y las lecturas quedan atascadas detrás de las escrituras. Falla alrededor del 42 % de las peticiones. «Caché fría tras un reinicio» arranca con todas las cachés vacías, algo que también nota un diseño que confía en la caché para proteger un almacén lento.

## Para probar

- Vuelve a usar PostgreSQL como almacén de pastes, conserva la Cache y ejecuta la ráfaga. Mira hasta dónde llegas solo con la caché.- Reduce o quita la Cache y compara el coste: menos aciertos significa más peticiones a Object Storage.
- Ejecuta «Caché fría tras un reinicio» y mira cómo sube la tasa de aciertos con el tiempo.
