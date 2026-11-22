## 1

¿Cuántos datos hay en las 50 clasificaciones juntas, y cuántas veces por segundo se leen?

## 2

Unos datos pequeños y calientes que se leen miles de veces por segundo deben vivir en memoria. Los conjuntos ordenados mantienen las puntuaciones en orden a medida que llegan, así que leer los 100 primeros es barato.

## 3

Usa Redis como almacén, sin TTL, y añade una réplica con una conexión Replicate y una conmutación por error corta. Pon detrás del Load Balancer suficientes App Servers en Go para soportar el doble del tráfico normal.
