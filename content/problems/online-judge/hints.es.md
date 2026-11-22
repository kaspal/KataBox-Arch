## 1

¿Qué le pasa a la cola, y a la espera de todos, cuando los envíos llegan más rápido de lo que los jueces pueden terminarlos?

## 2

Deja que el grupo de jueces crezca con la carga en lugar de dimensionarlo para una tarde tranquila. Una cola profunda mantiene a salvo los envíos; suficientes jueces mantienen corta la espera.

## 3

Pon los Judge Workers dentro de un Autoscaling group cuyo máximo sea varias veces el tamaño normal, con una utilización objetivo moderada y un retardo de escalado corto. Mantén delante una Queue SQS con un max depth muy grande, y un par de servidores de la Submit API detrás del Load Balancer.
