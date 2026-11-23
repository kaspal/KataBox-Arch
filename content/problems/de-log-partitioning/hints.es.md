## 1

Una búsqueda de las últimas 24 horas necesita 0,1 GB. ¿Cuánto tiene que leer el almacén para encontrarlos, y cuánto tiempo ocupa eso un hueco? Llegan dos consultas por segundo.

## 2

Una tabla se puede dividir por la columna que filtran las consultas, para que el almacén se salte las partes que no necesita. Cuando las búsquedas ya sean rápidas, vuelve a contar los huecos: una consulta del año entero sigue leyéndolo todo y ocupa su hueco más de medio minuto. ¿Qué les pasa a los huecos cuando una auditoría lanza muchas, o cuando todo el mundo busca a la vez?

## 3

Conecta Cliente (Tiempo límite 60 000 ms) → 2 Servidores de aplicaciones Go → Google BigQuery (1 instancia, Concurrencia por instancia 32) → Apache Iceberg con Particionado Por día, Compactación Diaria y Datos guardados (TB) 0,04.
