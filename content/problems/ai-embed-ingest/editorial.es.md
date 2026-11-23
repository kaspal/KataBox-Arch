# Solución comentada

## Adónde va la carga

5 documentos por segundo de unos 625 tokens cada uno son unos 3100 tokens por segundo, o unos 190 000 tokens por minuto: un tercio de los 600 000 que permite tu cuenta. El límite de peticiones queda lejos: 300 peticiones por minuto frente a 3000. Así que el límite que importa es el de tokens. En documentos, 600 000 tokens por minuto son 600 000 / 625 = 960 documentos por minuto, unos 16 por segundo.

El proveedor no cuenta por segundo. Te da lo permitido en un minuto de una vez y lo va rellenando sin parar, así que una ráfaga corta puede gastar tokens más rápido que el límite hasta que se acaba lo permitido. A partir de ahí, toda llamada por encima del ritmo de relleno recibe un 429.

Cada embedding le cuesta al proveedor unos 0,6 s para un documento típico: 500 ms antes de que empiece la respuesta y unos 0,1 s para leer 500 tokens a 5000 tokens por segundo. Quien hace la llamada conserva una plaza durante todo ese tiempo.

## El diseño de referencia

Un Cliente web envía las subidas a 2 App Servers Go. Cada uno publica el documento en una Cola Amazon SQS con Reintentos máximos en 10, y responde. Un Worker Celery con 2 instancias y una concurrencia de 12 consume la cola: llama a la tarjeta API de OpenAI (Tipo de modelo Embeddings, 3000 peticiones y 600 000 tokens por minuto) y escribe el vector en una Base de datos vectorial pgvector. Cuesta unos 20 971 $ al mes: 20 415 $ son tokens de embeddings, 260 $ la Base de datos vectorial, 140 $ los App Servers, otros 140 $ los Workers y 16 $ la cola. Un día normal tiene una latencia p95 hasta completar de unos 939 ms, y los Workers están ocupados en torno al 16 %.

¿Por qué 24 plazas de Worker? Un documento ocupa una plaza unos 0,74 s (los 100 ms del propio Worker, la llamada de embedding y la escritura del vector), así que 24 plazas tratan unos 32 documentos por segundo: el doble del límite sostenido de 16. Basta para gastar lo permitido en el minuto durante una ráfaga en vez de dejarlo sin usar. Más plazas no aportan nada: el techo es el límite del proveedor.

"Un cliente importa su archivo (×10 durante 45 s)" envía 50 documentos por segundo desde el segundo 20 hasta el 65, en una ejecución de 120 s. La cola los acepta todos al momento, y los Workers procesan unos 32 por segundo, gastando lo permitido más rápido de lo que se rellena. El resto espera en la cola, unos 800 documentos al acabar la importación, y quedan hechos unos 30 s después. Cuando se acaba lo permitido, las llamadas empiezan a recibir 429. Cada uno de esos mensajes vuelve a la cola y se reintenta tras 1 s, 2 s, 4 s y así sucesivamente, de modo que el 429 cuesta tiempo, no un documento. Ninguna subida falla, no se pierde nada y la latencia p95 hasta completar es de unos 32,5 s. Esta prueba permite una p95 de 60 s: una importación grande puede tardar algo más en poder buscarse, pero todos los documentos tienen que llegar.

"La API de embeddings se vuelve lenta" suma 5 s a cada embedding desde el segundo 20, durante 30 segundos. Cada documento ocupa ahora una plaza de Worker unos 5,7 s, así que las 24 plazas sacan unos 4,2 documentos por segundo mientras llegan 5. Se forma una pequeña cola de unas decenas de documentos que se vacía en segundos cuando el proveedor se recupera. Las subidas ni lo notan: los App Servers solo publican. La latencia p95 hasta completar es de unos 9,5 s, sin errores.

Este problema no tiene reto de coste: los tokens son el 97 % de la factura, y cada documento tiene que pasar una vez por el modelo al mismo precio. La prueba más barata que encontró la búsqueda cuesta 20 901 $, el 100 % de la referencia.

## Por qué falla el primer intento

Un primer intento habitual no tiene cola: los 2 App Servers Go llaman ellos mismos a la API de embeddings y a la Base de datos vectorial mientras la subida espera. Cuesta unos 20 815 $ y supera "Un día normal" con una latencia p95 de unos 802 ms.

La importación lo rompe. 50 documentos por segundo son unos 31 000 tokens por segundo, frente a un relleno de 10 000 por segundo, así que lo permitido en el minuto, 600 000 tokens, se acaba tras unos 600 000 / 21 000 ≈ 28 s. Desde el segundo 49 hasta el final de la importación, solo pasan unos 16 de cada 50 documentos por segundo, y el resto se rechaza con un 429. Nada los guarda para después, así que cada rechazo es una subida fallida: falla en torno al 15,9 % de los documentos de la ejecución.

La lentitud también lo rompe. Un embedding típico tarda ahora unos 5,6 s, más que el tiempo de espera de 5 s del Cliente web, así que todas las subidas de esos 30 segundos agotan el tiempo: falla en torno al 24,4 % de los documentos de la ejecución. A los App Servers no les faltan plazas (en un día normal están ocupados en torno al 3 %); la subida simplemente está esperando al modelo de otro.

La idea es esta: un proveedor lento y con límites va detrás de una cola. Un 429 significa "ahora no", así que dale al documento un sitio duradero donde esperar y reinténtalo con espera creciente. Dimensiona los Workers según los límites del proveedor, no según el tráfico: plazas suficientes para gastar lo permitido, y no más.

## Lo que el simulador deja fuera

Solo se miden la latencia, el rendimiento, los límites, el coste y los fallos. Si los embeddings son buenos, y si la búsqueda encuentra los documentos correctos, no se simula en absoluto. Tampoco se simulan el troceado de documentos largos, el análisis de los archivos ni la construcción del índice vectorial. El precio por token es el precio didáctico del registro para el sabor API de OpenAI, un precio de lista de un modelo de chat que aún está marcado como pendiente de fuente; consulta el precio de embeddings de tu proveedor. La ventana de entrega de la API por lotes no se simula como tiempo; en su lugar, una llamada por lotes para un trabajo que alguien está esperando (como aquí) falla. Los límites del proveedor son un cubo por minuto para las peticiones y otro para los tokens; los proveedores reales pueden contar de otra forma y enviar una cabecera Retry-After, que no se simula. La espera entre reintentos de la cola se duplica desde 1 s hasta 32 s y no se puede configurar. Un documento en una Cola de mensajes fallidos cuenta en tu contra, pero no se simula volver a enviarlo. En este simulador un modelo alojado no tiene límite de plazas, así que una lentitud gasta las plazas de tus Workers, no la capacidad del proveedor.

## Para probar

- Pon los Reintentos máximos de la cola en 3 y ejecuta la importación. Los documentos rechazados cuatro veces seguidas se pierden: unos 107. Conecta una Cola de mensajes fallidos a la cola y vuelve a ejecutarla: los mismos documentos esperan ahí, y la prueba sigue fallando, porque ninguno llegó a la búsqueda.
- Baja los Workers a una concurrencia de 6 (12 plazas) y ejecuta la importación. Los Workers no pueden gastar lo permitido durante la ráfaga, y la cola tarda más de un minuto en vaciarse.
- Sube los Workers a 8 instancias con una concurrencia de 16 y ejecuta la importación. La p95 no se mueve: el techo es el límite del proveedor, y las plazas extra solo recogen 429.
