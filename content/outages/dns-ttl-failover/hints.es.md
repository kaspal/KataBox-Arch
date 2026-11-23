## 1

Route 53 detectó en segundos que us-east no estaba y empezó a responder con us-west. Entonces, ¿por qué us-west nunca recibió una petición? Mira las propiedades de la tarjeta DNS.

## 2

Los clientes y los resolvers guardan una respuesta DNS durante su TTL, y solo vuelven a preguntar cuando caduca. Con un TTL de 86.400 s, siguen yendo a us-east durante un día. Y cuando por fin se mueven: ¿puede un solo App Server en us-west soportar 6000 peticiones por segundo (9000 en una hora con más tráfico)?

## 3

En Route 53, pon el TTL en 30 s y deja el enrutamiento en failover. En el App Server de us-west, pon 3 instancias (aquí un servidor Go soporta algo menos de 5000 peticiones por segundo). No hace falta cambiar nada más.
