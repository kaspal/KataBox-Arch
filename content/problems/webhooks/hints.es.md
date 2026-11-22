## 1

El endpoint de un cliente deja de responder durante un rato. ¿Qué pasa con los eventos que ibas a enviarle, y durante cuánto tiempo estás dispuesto a seguir intentándolo?

## 2

Reintenta las entregas fallidas con esperas cada vez más largas, el tiempo suficiente para aguantar una caída de un minuto o más, y conecta una cola de mensajes fallidos para que lo que siga fallando se guarde y se pueda reenviar en vez de perderse. Después cuenta los huecos de envío: cada llamada ocupa uno mientras el endpoint tarda en responder, y un endpoint caído lo ocupa un segundo entero solo para fallar.

## 3

Sube los reintentos máximos de la cola a unos 8 (las esperas son de 1, 2, 4, 8, 16, 32, 32… segundos, más de dos minutos en total), conecta una Dead-letter Queue desde ella y dales a los Senders Celery una concurrencia alta, de unos 64, porque enviar es sobre todo esperar a la red. Luego añade instancias hasta tener claramente más huecos que eventos por segundo multiplicados por los segundos que puede tardar cada llamada cuando los endpoints van lentos.
