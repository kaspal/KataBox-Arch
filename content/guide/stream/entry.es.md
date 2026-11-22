Un stream es un registro en el que solo se añade. Los productores añaden eventos al final, y el registro los guarda durante un tiempo fijado tanto si alguien los ha leído como si no. Cada consumidor recuerda su propia posición, así que muchos sistemas pueden leer los mismos eventos por separado, y uno nuevo puede empezar desde el principio.

Un stream se divide en **particiones**. Los eventos con la misma clave caen siempre en la misma partición, lo que los mantiene en orden, y cada partición la lee un solo consumidor de un grupo a la vez. Por eso el número de particiones es el techo de cuánto trabajo puede hacerse en paralelo.

Los streams brillan con grandes volúmenes de eventos que interesan a varias partes de un sistema: feeds de actividad, actualizaciones de posición, cambios capturados de una base de datos. Para una simple lista de tareas independientes, una cola suele ser más sencilla.
