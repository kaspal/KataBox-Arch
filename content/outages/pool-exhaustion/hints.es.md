## 1

Las vistas del carrito nunca llaman al servicio de recomendaciones, y aun así fallaron. ¿Qué estaban esperando en el App Server?

## 2

Cada página de producto retenía un slot del App Server mientras duraba la llamada a recomendaciones, y sin timeout de llamada eso eran 3 s. Un 30 % del tráfico esperando 3 s cada uno necesita muchos más de 256 slots. La página puede mostrarse sin recomendaciones, así que la app no tiene por qué esperar.

## 3

En el App Server, pon un timeout de llamada de 200 ms (el paso de recomendaciones es opcional, así que una llamada expirada sigue sin ellas) y activa el bulkhead, para que las llamadas a recomendaciones tengan su propia parte de los slots. No hace falta cambiar nada más.
