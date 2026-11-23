# Pipeline de logs y métricas

## El producto

Llevas la plataforma de logs de una empresa con cientos de servicios. Cada servicio reúne sus líneas de log y te las envía en lotes. Tu pipeline guarda cada lote en un archivo barato a largo plazo, y los ingenieros buscan en los logs cuando algo va mal. El flujo es grande y constante, y crece justo cuando algo se rompe: durante un incidente, todos los servicios escriben más. Un lote que nunca llega al archivo es un hueco en el registro de lo que pasó, así que no se puede perder ninguno.

## Lo que debes cumplir

| Objetivo | Valor |
| --- | --- |
| Tráfico pico | 1000 peticiones/s |
| Latencia p95 (los lotes se miden hasta quedar archivados) | 2 s o menos |
| Disponibilidad | 99,9 % o más |
| Presupuesto | 18 500 $ al mes |
| Mensajes perdidos | Ninguno |

## El tráfico

Los servicios llegan a una Collector, un App Server, normalmente a través de un Load Balancer.

- **Enviar un lote de líneas de log** (99 %). Cada lote ocupa unos 256 KB. La Collector publica el lote en un stream y responde enseguida. Un Worker lo toma del stream y lo escribe en un Object Store, el archivo. La latencia se mide hasta que el lote queda archivado, no hasta que responde la Collector.
- **Buscar en los logs** (1 %). Un ingeniero lanza una consulta y la Collector pide las líneas que coinciden a un índice Search.

No interviene ningún proveedor externo. Si el pipeline deja de enviar, los lotes que esperan en el stream siguen contando: cuando termina una prueba, cada lote aún sin archivar cuenta con la antigüedad que tenga en ese momento.

## Las pruebas

Hay una prueba visible. Puedes ejecutarla tantas veces como quieras:

- **Un día normal**: el tráfico descrito arriba, en su pico.

Hay dos pruebas ocultas más, de las que solo ves el nombre: **Un incidente triplica los logs** y **Cae un consumidor y el grupo se rebalancea**. Tu diseño tiene que superar las tres.
