## 1

En un día normal el clúster da abasto, así que el orden de la cola casi no importa. El día de lanzamiento los trabajos urgentes piden más tiempo de GPU del que tiene el clúster. ¿Quién debería esperar: la investigadora que bloquea un lanzamiento, o un barrido que nadie está mirando? ¿Y cuántas GPU necesita solo el trabajo urgente cuando falta un nodo?

## 2

Una cola FIFO (el primero en entrar es el primero en salir) hace que cada trabajo urgente espere detrás de todos los masivos que tiene delante, y cuando el trabajo supera a las GPU ese montón no deja de crecer. Haz que la cola entregue antes los trabajos urgentes, para que los masivos solo usen las GPU que sobran. Después dimensiona el clúster para la carga urgente en su pico con un nodo perdido, no para la carga media.

## 3

Mantén los Servidores de aplicaciones Go (2 instancias) y una Cola RabbitMQ con "Primero lo urgente (prioridad)" activado y un tiempo de espera de visibilidad de 60 s. Conéctala con Consumir a 5 nodos de GPU, cada uno un Worker Celery con 8 Instancias, "Concurrencia por instancia" 1 y un tiempo de servicio de 2000 ms.
