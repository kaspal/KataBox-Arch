# Solución comentada

## Adónde va la carga

De cada cien solicitudes, noventa y nueve piden un archivo que es igual para todos y que solo cambia con un despliegue. La popularidad está sesgada: unos pocos archivos, como la página de inicio y el script principal, se llevan una gran parte del tráfico. Eso hace que el trabajo sea fácil de guardar en caché. Solo el formulario de contacto, el 1 % del tráfico o unas 15 solicitudes por segundo, necesita un App Server y una base de datos.

Así que la pregunta real no es cuántos servidores necesitas. Es cuán pocas solicitudes deberían llegar a un servidor.

## El diseño de referencia

El Client habla solo con una CDN (CloudFront) con un TTL de un día. Detrás, un Object Storage (S3) guarda los archivos del sitio. Un acierto se responde en el borde en un par de milisegundos. Un fallo va una vez al Object Storage y llena la CDN, así que el siguiente visitante obtiene un acierto.

La CDN también reenvía el formulario de contacto a un Load Balancer, un App Server en Go y PostgreSQL. Con este volumen, un App Server basta de sobra.

El diseño cuesta unos 3 417 $ al mes. Casi todo son solicitudes a la CDN, que en este modelo cuestan 0,75 $ por millón. La p95 ronda los 7 ms.

Conecta el Client solo a la CDN. Si también lo conectas al Load Balancer, el tráfico se reparte y la mitad de las solicitudes de archivos se saltan la CDN.

## Por qué falla el primer intento

Un primer diseño habitual es Client, Load Balancer, dos App Servers en Go que sirven los archivos ellos mismos y PostgreSQL. Cuesta unos 515 $ y pasa un día normal con una p95 cercana a 17 ms.

La prueba oculta "El origen se vuelve lento" añade 300 ms a cada solicitud a App Server y a Object Storage entre los segundos 10 y 50. Ahora cada solicitud de archivo retiene un hilo de la aplicación durante 300 ms. Los hilos se agotan, las solicitudes hacen cola y cerca del 47 % falla. Con la CDN delante, la misma prueba no cambia nada visible, porque los aciertos nunca llegan al origen.

La otra prueba oculta, "Un despliegue purga la CDN", vacía la CDN en el segundo 20. Los fallos van al Object Storage mientras el borde se vuelve a llenar, y la p95 sube a unos 32 ms durante un rato, todavía por debajo del objetivo.

## Para probar

- Acorta el TTL a unos pocos segundos y observa cómo suben la carga del origen y la p95.
- Usa un App Server como origen en lugar del Object Storage y vuelve a ejecutar la prueba del origen lento.
- Conecta el Client a la CDN y al Load Balancer a la vez, y compara la tasa de aciertos.
