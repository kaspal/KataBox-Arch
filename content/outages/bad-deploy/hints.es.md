## 1

El error estaba en la versión, pero ¿cuánto de producción alcanzó antes de que nadie pudiera saber que era mala?

## 2

Todos los servidores recibieron la versión nueva en el mismo momento, y nada vigilaba los errores después, así que todo el servicio falló hasta que una persona la revirtió. Un despliegue debería llegar primero a unas pocas instancias, esperar y deshacerse solo cuando los health checks vean errores.

## 3

En el App Server, pon un despliegue del 10 %, un tiempo de horneado de 30 s y activa la reversión automática. No hace falta cambiar nada más.
