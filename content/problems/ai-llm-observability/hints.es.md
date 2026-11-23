## 1

¿Qué espera la persona del chat? ¿Su respuesta necesita que la traza ya esté en el almacén? Y si el almacén tiene un minuto lento, ¿quién debería notarlo: los paneles o la persona del chat?

## 2

Los paneles pueden ir minutos por detrás, así que la traza no tiene que llegar al almacén antes de que salga la respuesta. Pásala a otro: el Servidor de aplicaciones publica la traza en un Stream, que la guarda a salvo, y un consumidor la escribe en el almacén a su ritmo. Así, un almacén lento solo retrasa un poco los paneles. Dimensiona las particiones del Stream y el consumidor para el lanzamiento: cada escritura ocupa un hueco del consumidor mientras tarde el almacén.

## 3

Pon un AWS ALB delante de 4 Servidores de aplicaciones Go, con el Tiempo límite del Cliente web en 30 000 ms. Los Servidores de aplicaciones llaman a una API de Claude con el Límite de peticiones por minuto en 4000 y el Límite de tokens por minuto en 2 000 000, y publican cada traza en un Stream Apache Kafka con 12 particiones. Un Procesador de streams Apache Flink (2 instancias, Concurrencia por instancia 8) lo consume y escribe las trazas en un Almacén de datos Snowflake con sus ajustes por defecto.
