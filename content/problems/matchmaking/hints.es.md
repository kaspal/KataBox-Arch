## 1

Una cola permite que los jugadores esperen en lugar de fallar, pero solo sirve si los emparejadores pueden ponerse al día. ¿Cómo de ocupados están tus emparejadores en el pico, y qué le pasa a la cola cuando llega el doble de jugadores?

## 2

Los jugadores quieren una partida cerca de ellos, así que el emparejamiento se divide de forma natural por región. Una pila completa en cada región, con su propia cola y sus propios emparejadores, reduce a la mitad la carga que ve cada una, y si se pierde una región, el DNS puede mandar a sus jugadores a la otra.

## 3

Pon un DNS Route 53 (TTL de 30 s) delante de dos regiones, us-east y eu-west. En cada una, conecta un AWS ALB a unos Go App Servers con 2 instancias, una cola RabbitMQ, emparejadores Celery con 4 instancias, concurrencia 64 y un tiempo de servicio de 200 ms, y una tabla DynamoDB. Cada región empareja a sus propios jugadores.
