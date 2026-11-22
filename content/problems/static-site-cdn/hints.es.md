## 1

El sitio tiene 20 000 archivos que casi nunca cambian. ¿De verdad un servidor tiene que buscar el mismo archivo cada vez que alguien abre una página?

## 2

Guarda los archivos en caché en el borde, cerca de los visitantes. Cuando la mayoría de las solicitudes se responden allí, el origen sale del camino caliente, y sus problemas dejan de llegar a tus visitantes.

## 3

Pon una CDN con un TTL largo delante de un Object Storage que guarde los archivos del sitio, y conecta el Client solo a la CDN. Deja que la CDN reenvíe el formulario de contacto a un Load Balancer y a un App Server pequeño que escriba en la base de datos.
