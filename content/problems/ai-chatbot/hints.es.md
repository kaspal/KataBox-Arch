## 1

El proveedor cuenta los tokens por minuto y por clave, y tu nivel ya es el más alto. ¿Cuántos tokens por minuto envía tu pico, y cuántos enviará el lanzamiento? Y cuando el único proveedor al que llamas deja de responder, o empieza a rechazar llamadas, ¿adónde va un mensaje?

## 2

Con 4 veces el tráfico, una clave de 1 000 000 de tokens por minuto se queda sin tokens tras unos dos minutos, y a partir de ahí se rechazan muchas llamadas hasta que se recarga lo que se permite por minuto. No hay un nivel más alto, así que añade capacidad con la que puedas contar: más claves. Un segundo proveedor es también un sitio al que mandar mensajes cuando el primero falla o te recorta el límite, y un cliente que lo vuelve a intentar tras una espera corta y aleatoria convierte una llamada rechazada o fallida en una respuesta más lenta en vez de en un error. Las comprobaciones de salud notan un proveedor caído; no notan uno que responde con un 429.

## 3

Pon un AWS ALB delante de 2 Servidores de aplicaciones Go. Conéctalos a dos tarjetas de Inferencia de modelos: una API de Claude y una Amazon Bedrock. En el Cliente web, pon el Tiempo límite en 30 000 ms, los Reintentos en 3, la Espera entre reintentos en 200 ms y la Variación de la espera en Completa (espera aleatoria).
