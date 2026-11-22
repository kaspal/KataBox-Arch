## 1

Cuando no hay nadie disponible para ejecutar una tarea, ¿dónde espera esa tarea, y cuántas pueden esperar allí antes de que se rechace alguna?

## 2

Usa una cola profunda y duradera como amortiguador entre el planificador y los workers, y deja que el grupo de workers crezca con la acumulación en lugar de mantener un tamaño fijo.

## 3

Usa una Queue SQS con un max depth muy grande, para que nunca rechace una tarea, y pon los Workers dentro de un Autoscaling group con un mínimo pequeño, un máximo amplio, una utilización objetivo moderada y un retardo de escalado corto. Una base de datos PostgreSQL basta para los resultados.
