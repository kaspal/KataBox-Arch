# Solución comentada

## Adónde va la carga

Los Servidores de aplicaciones hacen poco: pasan cada mensaje y esperan la respuesta. Los límites de verdad son los del proveedor, y se cuentan en tokens por minuto y por clave.

Un mensaje envía unos 800 tokens y recibe unos 300: 1100 tokens. En el pico de 5 mensajes por segundo son 5 × 60 × 1100 = 330 000 tokens por minuto, el 82 % de los 400 000 de una clave. Una clave basta para un día normal. El límite de peticiones no es el problema: 300 peticiones por minuto frente a 4000.

El lanzamiento multiplica eso por 4: 1 320 000 tokens por minuto. Una clave guarda lo que se permite en un minuto y lo va recargando a lo largo del minuto, así que una sola clave de 400 000 por minuto se vacía tras unos 400 000 / (1 320 000 − 400 000) de minuto, unos 26 segundos. A partir de ahí solo deja pasar 400 000 de cada 1 320 000 tokens, y el resto se rechaza con un 429.

La prueba de saturación es distinta: a partir del segundo 20, durante 20 segundos, una tarjeta de Inferencia de modelos falla todas las llamadas. Con un solo proveedor, falla cada mensaje de esos 20 segundos.

## El diseño de referencia

Un Cliente web con un Tiempo límite de 30 000 ms, 3 Reintentos, una Espera entre reintentos de 200 ms y la Variación de la espera en Completa (espera aleatoria) envía los mensajes a un AWS ALB, que los reparte entre 2 Servidores de aplicaciones Go. Estos llaman por turnos a dos tarjetas de Inferencia de modelos: una API de Claude y una Amazon Bedrock, cada una con el Límite de tokens por minuto subido a 1 000 000. Cuesta unos 93 747 $ al mes, y los tokens son casi todo: unos 46 805 $ en la API de Claude y 46 777 $ en Bedrock, 140 $ de los Servidores de aplicaciones y 25 $ del ALB. Dos claves no cuestan más que una: la factura es por token, y los tokens son los mismos. Un día normal tiene un tiempo hasta el primer token (TTFT) p95 de 1,0 s más o menos, un p95 de la respuesta entera de unos 8,6 s, y ningún error.

«Día de lanzamiento» multiplica el tráfico por 4 a partir del segundo 30, durante 3 minutos, en una ejecución de 4 minutos. Un lanzamiento real dura horas; 3 minutos bastan para gastar lo que haya guardado, así que lo que queda es el ritmo sostenido. Las dos claves aceptan juntas 2 000 000 de tokens por minuto, y el lanzamiento usa el 66 %. No hay errores, el TTFT p95 se queda en 1,0 s más o menos y la respuesta entera en unos 8,1 s.

«El proveedor del modelo se satura durante 20 s» detiene una tarjeta de Inferencia de modelos a partir del segundo 20, durante 20 segundos, en una ejecución de 90 segundos. Una llamada a ella falla al instante. Durante 2 a 4 segundos, hasta que dos comprobaciones de salud seguidas la encuentran caída, los Servidores de aplicaciones siguen mandándole la mitad de los mensajes; después, todos los mensajes van al otro proveedor, que tiene mucho margen con el tráfico normal (330 000 de sus 1 000 000 de tokens por minuto). El cliente vuelve a intentar los mensajes que fallaron en esos segundos tras una espera aleatoria de hasta 200 ms, y los Servidores de aplicaciones alternan entre las dos tarjetas, así que con 3 reintentos uno de ellos llega al proveedor que sigue funcionando. No hay errores, el TTFT p95 es de 1,0 s más o menos y la respuesta entera de unos 9,5 s.

Este problema no tiene reto de coste. Los tokens son más del 99 % de la factura y los fija el tráfico, así que el diseño más barato encontrado que supera todas las pruebas cuesta lo mismo que la referencia.

## Por qué falla el primer intento

Un primer intento habitual es un Cliente web con el Tiempo límite en 30 000 ms y sin reintentos, el mismo ALB y 2 Servidores de aplicaciones, y una sola API de Claude en su nivel actual. Cuesta los mismos 93 747 $ y supera «Un día normal» con los mismos números.

El día del lanzamiento falla con un 53,7 % de errores más o menos. Unos 22 segundos después de empezar el lanzamiento, se acaba el minuto guardado de la clave, y desde entonces la mayoría de las llamadas se rechazan con un 429, segundo tras segundo, hasta que el lanzamiento termina. Los Servidores de aplicaciones no son el problema: en un día normal solo están ocupados un 17 % más o menos. El límite del proveedor sí lo es.

La saturación da un 28,1 % de errores más o menos: cada mensaje de esos 20 segundos falla al instante, porque no hay otro sitio al que mandarlo.

Arreglar una sola mitad no basta:

- Una API de Claude subida a 1 000 000 de tokens por minuto, con los mismos reintentos, supera el lanzamiento (un 4,5 % de errores más o menos) y sigue fallando la saturación con un 26,7 % de errores más o menos. Los reintentos con una espera de 200 ms esperan como mucho 1,4 s en total; no pueden aguantar 20 segundos.
- Dos claves en el nivel actual, con reintentos, superan la saturación y fallan el lanzamiento con un 15,9 % de errores más o menos: 800 000 tokens por minuto son menos que los 1 320 000 del lanzamiento.
- Las dos claves subidas de la referencia sin reintentos fallan la saturación con un 2,9 % de errores más o menos: los mensajes enviados a la tarjeta caída antes de que las comprobaciones de salud lo notaran.

Tres claves en el nivel actual con reintentos superan todas las pruebas, pero por poco: 1 200 000 tokens por minuto es algo menos de lo que necesita el lanzamiento, y el diseño aguanta gracias al minuto guardado de cada clave. Un lanzamiento más largo lo rompería.

La idea es esta: los límites de un modelo alojado forman parte de tu plan de capacidad. Calcula los tokens por minuto de tu pico y de tu lanzamiento, y ten claves y un nivel que los cubran a un ritmo sostenido, no por suerte. Y un solo proveedor es un punto único de fallo: un segundo proveedor, más reintentos en el cliente con una espera que crece y es aleatoria, convierten su mal día en respuestas algo más lentas.

## Lo que el simulador deja fuera

La calidad de las respuestas no se simula: para el simulador, las tarjetas API de Claude y Bedrock dan las mismas respuestas, a la misma velocidad y al mismo precio. Un segundo proveedor real puede necesitar otro prompt y dar respuestas distintas. Subir de nivel aquí es una propiedad, gratis e instantánea; en la realidad lo decide el proveedor, a menudo según tu historial de gasto, y puede llevar tiempo. Una saturación real suele devolver errores 429 o 529 en parte de las llamadas en vez de fallarlas todas; aquí la tarjeta simplemente cae. Las cabeceras Retry-After, la caché de prompts, los historiales de conversación que crecen y la cola propia del proveedor no se simulan. El streaming solo se modela como tiempos: el tiempo hasta el primer token y hasta el último. Los precios por token son las cifras didácticas del registro.

## Para probar

- Quita la tarjeta Bedrock y ejecuta «Día de lanzamiento» y la saturación. ¿Qué prueba falla, y por qué el nivel más alto no puede ayudar con ella?
- Pon los Reintentos del Cliente web en 0 y ejecuta la saturación. Mira sus primeros segundos.
- Vuelve a poner las dos claves en 400 000 tokens por minuto y ejecuta «Día de lanzamiento». Busca el segundo en que empiezan los errores.
