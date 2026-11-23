## 1

El proveedor cuenta los tokens por minuto y por clave. ¿Cuántos tokens por minuto envía tu pico, y cuántos enviará el lanzamiento? Y cuando el único proveedor al que llamas deja de responder, ¿adónde va un mensaje?

## 2

Con 4 veces el tráfico, una clave en su nivel actual se queda sin tokens en menos de un minuto, y a partir de ahí se rechazan todas las llamadas hasta que se recarga lo que se permite por minuto. Añade capacidad con la que puedas contar: más claves, un nivel más alto, o las dos cosas. Un segundo proveedor es también un sitio al que mandar mensajes cuando el primero falla, y un cliente que lo vuelve a intentar tras una espera corta y aleatoria convierte una llamada rechazada o fallida en una respuesta más lenta en vez de en un error.

## 3

Pon un AWS ALB delante de 2 Servidores de aplicaciones Go. Conéctalos a dos tarjetas de Inferencia de modelos: una API de Claude y una Amazon Bedrock, cada una con el Límite de tokens por minuto subido a 1 000 000. En el Cliente web, pon el Tiempo límite en 30 000 ms, los Reintentos en 3, la Espera entre reintentos en 200 ms y la Variación de la espera en Completa (espera aleatoria).
