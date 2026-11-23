## 1

Calcula el tamaño del estado del procesador: cuántas sesiones hay abiertas en una ventana de 60 s, por 128 KB cada una. Luego compáralo con la memoria que le diste al procesador. Y pregúntate qué le pasa a la tabla de sesiones cuando el procesador se reinicia y tiene que rehacer el trabajo desde la última vez que guardó su progreso.

## 2

400 clics por segundo en una ventana de 60 s son 24 000 sesiones abiertas; a 128 KB cada una son unos 2,9 GB. Si eso supera Memoria × Instancias, cada registro es 4 veces más lento: aguanta un día tranquilo, pero no cuando crecen los usuarios. En una caída, el procesador vuelve a su último checkpoint y escribe otra vez todo lo posterior, así que el Intervalo de checkpoint decide cuánto se repite. Las escrituras que hacen upsert por clave vuelven inofensivas las repeticiones.

## 3

Usa un Stream de Apache Kafka con 12 particiones y un Procesador de streams Apache Flink con 3 instancias, Memoria de 8 GB, Intervalo de checkpoint de 10 s, Ventana `sliding`, Duración de la ventana de 60 s, Desplazamiento de 1 s, Retraso de la marca de agua de 5 s y Estado por clave de 128 KB. Escribe en un Almacén NoSQL ScyllaDB con "Escrituras idempotentes (dedup)" activado. Deja 2 Servidores de aplicaciones Go delante.
