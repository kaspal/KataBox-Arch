## 1

Nada falló, y la avalancha duró solo 30 s, pero tres minutos después los pedidos seguían llegando con más de un minuto de retraso. ¿A qué ritmo pueden los workers despachar lo que se acumuló en la cola?

## 2

Los workers procesan 160 pedidos por segundo; la avalancha trajo 500. Todo lo que pasaba de 160 esperaba en una cola que guardaba diez millones, y cada pedido nuevo esperaba detrás de todo eso. Una cola solo ayuda si se vacía: limita cuánto guarda, para que el exceso se rechace al momento, y dale más workers cuando se llene.

## 3

En la cola, pon la profundidad máxima en 3000. Mete los workers en un grupo Autoscale (Kubernetes HPA) con 2 a 6 instancias, un objetivo del 50 % y un retraso de escalado de 5 s.
