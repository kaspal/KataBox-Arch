# Planificador de tareas distribuido

## El producto

Tienes el servicio que ejecuta el trabajo en segundo plano de todos los demás. Los equipos registran tareas que deben ocurrir según un calendario: enviar el informe diario, reconstruir un índice de búsqueda, expirar sesiones antiguas, renovar una suscripción. Un planificador lanza cada tarea a su hora y la entrega. Nadie mira una pantalla mientras la tarea se ejecuta, pero cada tarea tiene que ejecutarse, y tiene que terminar razonablemente pronto después de lanzarse. Una tarea que desaparece sin avisar es lo peor que puede hacer este servicio.

## Lo que debes cumplir

| Objetivo | Valor |
| --- | --- |
| Tráfico pico | 100 tareas/s |
| Tiempo de finalización p95 | 10 s o menos |
| Disponibilidad | 99,9 % o más |
| Presupuesto | 2 500 $ al mes |
| Tareas perdidas | ninguna |

Aquí la latencia se mide hasta que la tarea ha terminado y su resultado está guardado, no hasta que se entrega.

## El tráfico

- **Ejecutar una tarea programada** (100 %). Un Scheduler (el cron) publica cada tarea en una Queue. Los Workers toman tareas de la Queue y las ejecutan; cada tarea requiere unos 100 ms de trabajo. Cuando termina, el Worker escribe el resultado en una base de datos.

El Scheduler no frena cuando el sistema que tiene detrás está ocupado. Las tareas se siguen lanzando a su hora, haya o no alguien listo para ejecutarlas.

## Las pruebas

Hay una prueba visible. Puedes ejecutarla tantas veces como quieras:

- **Una hora normal**: el tráfico descrito arriba, en su pico.

Hay dos pruebas ocultas más, de las que solo ves el nombre: **Se pierde el grupo de workers** y **Medianoche**. Tu diseño tiene que superar las tres.
