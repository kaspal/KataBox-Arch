## 1

El proveedor decide a qué velocidad puedes enviar, y puede cambiar de opinión. ¿Qué pasa con un correo que rechaza, y cuánto tiempo puede esperar ese correo a otro intento?

## 2

Una cola que reintenta con esperas cada vez más largas reparte la carga en el tiempo: cuando el proveedor acepta menos llamadas, los correos rechazados vuelven un poco más tarde y entonces pasan. Un límite más bajo debería significar una campaña más lenta, no correo perdido, siempre que los reintentos duren más que el límite. Conecta una cola de mensajes fallidos para que lo que siga fallando se guarde en vez de descartarse.

## 3

Sube los reintentos máximos de la cola a unos 10 (las esperas son de 1, 2, 4, 8, 16, 32, 32… segundos, más de tres minutos en total) y conecta una Dead-letter Queue desde ella. Los Mail senders Celery con una concurrencia alta, de unos 64, encajan en un trabajo que consiste sobre todo en esperar al proveedor; 4 instancias de 64 bastan de sobra para 500 correos por segundo.
