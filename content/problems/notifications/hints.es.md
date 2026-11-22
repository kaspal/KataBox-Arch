## 1

Un proveedor está caído y una notificación no se puede enviar. ¿Qué le pasa después, y qué pasa si vuelve a fallar, una y otra vez?

## 2

Reintenta los mensajes fallidos con esperas cada vez más largas entre intentos, y guarda los que sigan fallando en un lugar seguro, una cola de mensajes fallidos, para poder reenviarlos más tarde en vez de perderlos.

## 3

Sube los reintentos máximos de la cola hasta que las esperas entre intentos sumen más que una caída larga, conecta una Dead-letter Queue desde la cola y dales a los Workers muchos huecos de envío, con muchas instancias y alta concurrencia, para que un proveedor caído no pueda ocuparlos todos.
