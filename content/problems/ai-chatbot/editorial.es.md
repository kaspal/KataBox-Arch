# Solución comentada

## Adónde va la carga

Los Servidores de aplicaciones hacen poco: pasan cada mensaje y esperan la respuesta. Los límites de verdad son los del proveedor, y se cuentan en tokens por minuto y por clave. Tu cuenta ya está en el nivel más alto, 1 000 000 de tokens por minuto por clave, así que la única forma de tener más es tener más claves.

Un mensaje envía unos 800 tokens y recibe unos 300: 1100 tokens. En el pico de 5 mensajes por segundo son 5 × 60 × 1100 = 330 000 tokens por minuto, el 33 % del 1 000 000 de una clave. Una clave sobra para un día normal. El límite de peticiones no es el problema: 300 peticiones por minuto frente a 4000.

El lanzamiento multiplica eso por 4: 1 320 000 tokens por minuto, más de lo que permite una clave. Una clave guarda lo que se permite en un minuto y lo va recargando a lo largo del minuto, así que una sola clave se vacía tras unos 1 000 000 / (1 320 000 − 1 000 000) de minuto, algo más de 3 minutos según la cuenta. A partir de ahí solo deja pasar 1 000 000 de cada 1 320 000 tokens, y el resto se rechaza con un 429.

La prueba de saturación es distinta: a partir del segundo 20, durante 20 segundos, una tarjeta de Inferencia de modelos falla todas las llamadas. Con un solo proveedor, falla cada mensaje de esos 20 segundos.

El recorte del límite es distinto otra vez: a partir del segundo 20, durante 40 segundos, el proveedor recorta el límite de una tarjeta a 50 000 tokens por minuto, y con él lo que tenía guardado. La tarjeta sigue en pie y sigue respondiendo, así que las comprobaciones de salud no ven nada raro; simplemente rechaza la mayoría de las llamadas con un 429.

## El diseño de referencia

Un Cliente web con un Tiempo límite de 30 000 ms, 3 Reintentos, una Espera entre reintentos de 200 ms y la Variación de la espera en Completa (espera aleatoria) envía los mensajes a un AWS ALB, que los reparte entre 2 Servidores de aplicaciones Go. Estos llaman por turnos a dos tarjetas de Inferencia de modelos: una API de Claude y una Amazon Bedrock, cada una en el nivel del lanzamiento, de 1 000 000 de tokens por minuto. Cuesta unos 62 553 $ al mes, y los tokens son casi todo: unos 31 204 $ en la API de Claude y 31 185 $ en Bedrock, 140 $ de los Servidores de aplicaciones y 25 $ del ALB. Dos claves no cuestan más que una: la factura es por token, y los tokens son los mismos. Un día normal tiene un tiempo hasta el primer token (TTFT) p95 de 1,45 s más o menos, un p95 de la respuesta entera de unos 7,5 s, y ningún error.

«Día de lanzamiento» multiplica el tráfico por 4 a partir del segundo 30, durante 4 minutos, en una ejecución de 5 minutos. Un lanzamiento real dura horas; 4 minutos bastan para gastar lo que tenga guardado una clave, así que lo que queda es el ritmo sostenido. Las dos claves aceptan juntas 2 000 000 de tokens por minuto, y el lanzamiento usa el 66 %. No hay errores, el TTFT p95 es de 1,43 s más o menos y la respuesta entera de unos 7,1 s.

«El proveedor del modelo se satura durante 20 s» detiene una tarjeta de Inferencia de modelos a partir del segundo 20, durante 20 segundos, en una ejecución de 90 segundos. Una llamada a ella falla al instante. Durante 2 a 4 segundos, hasta que dos comprobaciones de salud seguidas la encuentran caída, los Servidores de aplicaciones siguen mandándole la mitad de los mensajes; después, todos los mensajes van al otro proveedor, que tiene mucho margen con el tráfico normal (330 000 de su 1 000 000 de tokens por minuto). El cliente vuelve a intentar los mensajes que fallaron en esos segundos tras una espera aleatoria de hasta 200 ms, y los Servidores de aplicaciones alternan entre las dos tarjetas, así que con 3 reintentos casi siempre uno de ellos llega al proveedor que sigue funcionando. Falla un 0,2 % de los mensajes más o menos, el TTFT p95 es de 1,5 s más o menos y la respuesta entera de unos 8,8 s.

«El proveedor del modelo te recorta el límite durante 40 s» recorta la tarjeta API de Claude a 50 000 tokens por minuto a partir del segundo 20, durante 40 segundos, en una ejecución de 90 segundos. Los Servidores de aplicaciones le siguen mandando la mitad de los mensajes, unos 165 000 tokens por minuto, porque la tarjeta no está caída. Sus 50 000 tokens guardados duran unos 20 segundos; después rechaza unas dos llamadas de cada tres. El cliente vuelve a intentar cada mensaje rechazado, y el siguiente intento suele ir a Bedrock, que no tiene recorte. Falla un 0,7 % de los mensajes más o menos, el TTFT p95 es de 1,5 s más o menos y la respuesta entera de unos 7,4 s.

Este problema no tiene reto de coste. Los tokens son más del 99 % de la factura y los fija el tráfico, así que el diseño más barato encontrado que supera todas las pruebas cuesta lo mismo que la referencia.

## Por qué falla el primer intento

Un primer intento habitual es un Cliente web con el Tiempo límite en 30 000 ms y sin reintentos, el mismo ALB y 2 Servidores de aplicaciones, y una sola API de Claude. Cuesta los mismos 62 553 $ y supera «Un día normal» con los mismos números.

El día del lanzamiento falla con un 12,0 % de errores más o menos. Unos 2 minutos después de empezar el lanzamiento, se acaba el minuto guardado de la clave, y desde entonces se rechazan con un 429 unas 3 llamadas de cada 10, segundo tras segundo, hasta que el lanzamiento termina. Es antes de lo que dice la cuenta de arriba: los tokens del enunciado son valores típicos, y los mensajes largos suben la media. Los Servidores de aplicaciones no son el problema: en un día normal solo están ocupados un 16 % más o menos. El límite del proveedor sí lo es.

La saturación da un 27,9 % de errores más o menos: cada mensaje de esos 20 segundos falla al instante, porque no hay otro sitio al que mandarlo. El recorte del límite da un 28,3 % de errores más o menos: unos 8 segundos después del recorte, lo guardado en la clave se acaba, y desde entonces se rechaza la mayoría de los mensajes hasta que termina el recorte.

Arreglar una sola mitad no basta:

- Una sola API de Claude con los mismos reintentos falla las tres pruebas ocultas: el lanzamiento con un 7,4 % de errores más o menos, la saturación con un 26,5 % y el recorte con un 26,7 %. Cada reintento vuelve a la misma clave, y los reintentos con una espera de 200 ms esperan como mucho 1,4 s en total; no pueden aguantar una clave que se queda corta durante minutos, un proveedor caído durante 20 segundos ni un recorte que dura 40.
- Los dos proveedores de la referencia sin reintentos superan el lanzamiento y fallan las otras dos. La saturación da un 2,7 % de errores más o menos: los mensajes enviados a la tarjeta caída antes de que las comprobaciones de salud lo notaran. El recorte da un 7,3 %: las comprobaciones de salud nunca notan un recorte, así que cualquier mensaje que los Servidores de aplicaciones mandan a la API de Claude cuando se le acaba lo guardado puede ser rechazado, y nadie lo vuelve a intentar.

La idea es esta: los límites de un modelo alojado forman parte de tu plan de capacidad. Calcula los tokens por minuto de tu pico y de tu lanzamiento, y ten claves suficientes para cubrirlos a un ritmo sostenido, no por suerte. Y un solo proveedor es un punto único de fallo: un segundo proveedor, más reintentos en el cliente con una espera que crece y es aleatoria, convierten su mal día, esté caído o solo rechazando llamadas, en respuestas algo más lentas.

## Lo que el simulador deja fuera

La calidad de las respuestas no se simula: para el simulador, las tarjetas API de Claude y Bedrock dan las mismas respuestas, a la misma velocidad y al mismo precio. Un segundo proveedor real puede necesitar otro prompt y dar respuestas distintas. Aquí el nivel viene dado; en la realidad lo decide el proveedor, a menudo según tu historial de gasto, y subirlo puede llevar tiempo. Una saturación real suele devolver errores 429 o 529 en parte de las llamadas en vez de fallarlas todas; aquí la tarjeta simplemente cae. La saturación y el recorte afectan cada uno a una tarjeta; el mal día de un proveedor real afecta a todas las claves que tengas con él. Las cabeceras Retry-After, la caché de prompts, los historiales de conversación que crecen y la cola propia del proveedor no se simulan. El streaming solo se modela como tiempos: el tiempo hasta el primer token y hasta el último. Los precios por token son las cifras didácticas del registro.

## Para probar

- Quita la tarjeta Bedrock y ejecuta «Día de lanzamiento». Busca el segundo en que empiezan los errores y compáralo con la cuenta de arriba.
- Pon los Reintentos del Cliente web en 0 y ejecuta el recorte del límite. Busca el segundo en que empiezan los errores.
- Pon los Reintentos del Cliente web en 0 y ejecuta la saturación. Mira sus primeros segundos.
