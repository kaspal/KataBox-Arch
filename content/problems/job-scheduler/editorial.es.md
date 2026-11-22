# Solución comentada

## Adónde va la carga

Con 100 tareas/s y unos 100 ms cada una, el trabajo constante mantiene ocupados unos 10 huecos de Worker en cada momento. Es poco. La dificultad no está en la media, sino en los momentos en que la demanda y la capacidad dejan de coincidir: cuando las tareas llegan más rápido de lo que los Workers pueden ejecutarlas, o cuando no hay ningún Worker. El Scheduler nunca espera, así que cada tarea que lanza tiene que guardarse en algún sitio hasta que un Worker quede libre. Ese sitio es la Queue, y su profundidad decide si un mal minuto significa tareas con retraso o tareas perdidas. El retraso está permitido. Perderlas, no.

## El diseño de referencia

El Scheduler publica en una Queue SQS con un max depth de 1 000 000. La consumen Workers Celery dentro de un Autoscaling group con un mínimo de 2, un máximo de 24, una utilización objetivo del 60 % y un retardo de escalado de 10 segundos. Cada Worker escribe su resultado en PostgreSQL, que maneja sin esfuerzo 100 escrituras pequeñas por segundo.

Cuesta unos 759 $ al mes. En una hora normal, el tiempo de finalización p95 ronda los 0,3 s. Cuando todo el grupo de workers cae durante 20 segundos, las tareas se acumulan en la Queue, no se pierde ninguna y el p95 sube a unos 19 s mientras se vacía la acumulación. A medianoche, cuando llegan 10 veces más tareas durante 10 segundos, la Queue absorbe la ráfaga, el grupo escala y el p95 llega a unos 24 s. Las dos pruebas ocultas admiten un p95 de hasta 40 s.

## Por qué falla el primer intento

Un primer intento habitual usa una Queue RabbitMQ con un max depth de 1 000 y 2 Workers fijos. En una hora normal funciona bien. Cuando el grupo se pierde durante 20 segundos a partir del segundo 20, llegan unas 2 000 tareas sin nadie que las tome. La Queue se llena al llegar a 1 000, y unas 980 tareas se rechazan y se pierden. A medianoche, los 2 Workers no pueden seguir el ritmo de 1 000 tareas por segundo, la Queue vuelve a llenarse y se pierden unas 7 800 tareas.

## Para probar

- Mantén el Autoscaling group, pero vuelve a poner el max depth en 1 000. ¿Qué prueba sigue perdiendo tareas?
- Sube el retardo de escalado a 30 segundos y observa el p95 a medianoche.
- Baja el máximo a 8 instancias y mira cuánto tarda en vaciarse la acumulación de medianoche.
