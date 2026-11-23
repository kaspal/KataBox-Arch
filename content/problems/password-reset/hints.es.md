## 1

Aquí pueden fallar dos cosas que no tienen nada que ver con la velocidad. El proveedor de correo puede dejar de responder un rato, y el mismo enlace se puede abrir dos veces en el mismo momento. ¿Dónde se guarda un correo de restablecimiento mientras el proveedor no está? ¿Y qué impide que dos aperturas del mismo enlace encuentren las dos el token sin usar?

## 2

Un correo de restablecimiento es un trabajo que puede esperar. Si el App Server lo entrega a una cola que reintenta con esperas cada vez más largas, una caída se convierte en un retraso en vez de un correo perdido, siempre que los reintentos duren más que la caída. Para el token, «leerlo y después escribirlo» deja un hueco: dos canjes pueden leer «sin usar» antes de que ninguno escriba. La comprobación y la escritura tienen que ir en un solo paso, para que el segundo se rechace.

## 3

Conecta los App Servers a una cola RabbitMQ con Reintentos máximos en 10 (las esperas son de 1, 2, 4, 8, 16, 32, 32… segundos, más de tres minutos en total), y haz que 4 instancias Celery de Mailer con concurrencia 32 la consuman y llamen al proveedor. En la tarjeta PostgreSQL, activa «Escrituras condicionales (comprobar y escribir en un paso)». Para lo demás bastan 2 Go App Servers detrás de un AWS ALB.
