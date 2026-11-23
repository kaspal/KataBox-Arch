## 1

El límite de filas duplicadas es cero, no "unas pocas". Pregúntate qué hace el procesador de streams cuando se reinicia tras una caída, y qué ve el libro. Luego pregúntate qué ajuste lo hace inofensivo en vez de solo más pequeño. Calcula también cuántas escrituras hay en curso a la vez cuando cada escritura tarda 300 ms, y en una tormenta de tres veces los eventos.

## 2

En una caída, el procesador vuelve a su último checkpoint y escribe otra vez cada evento posterior. Un Intervalo de checkpoint más corto solo hace esa ventana más pequeña: incluso con 1 s algunos eventos llegan dos veces. Las escrituras con el id del evento como clave hacen que una segunda escritura del mismo evento no cambie nada, así que la repetición cuesta trabajo pero no duplicados. Para la capacidad: 120 eventos/s × 0,3 s son 36 escrituras en curso, antes del atasco que deja la caída; dale al procesador huecos, al Stream particiones y al almacén nodos para más que eso.

## 3

Mantén 2 servidores de aplicación en Go y un Stream Apache Kafka con 64 particiones. Usa un Procesador de streams Apache Flink con 2 instancias, Concurrencia por instancia 32 e Intervalo de checkpoint 10 s. Escribe en un Almacén de datos ClickHouse con 8 instancias y "Escrituras idempotentes (dedup)" activado.
