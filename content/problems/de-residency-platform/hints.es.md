## 1

Nada en el diseño distingue un evento de la UE de uno de EE. UU., así que cualquier sitio donde pueda acabar un evento tiene que valer para los dos. Haz la lista de las tarjetas que guardan un evento: el stream, la tabla de consentimientos, el lago. Después pregúntate adónde va cada una en un mal día, cuando se pierde una región o falla un primario.

## 2

Un respaldo también forma parte del pipeline. Si la ingesta conmuta a una región de EE. UU., o la réplica de la tabla de consentimientos está en EE. UU., el primer fallo saca datos de la UE fuera de la UE. Monta el pipeline entero en dos regiones etiquetadas las dos `eu`, y deja la réplica de la tabla de consentimientos en la otra región de la UE. Solo el almacén de datos con los agregados va en EE. UU. Cada región de la UE tiene que aguantar el pico entero ella sola.

## 3

Un Cliente con 2 reintentos, una espera entre reintentos de 200 ms y la variación de la espera en Completa llama a una tarjeta DNS Route 53 (TTL de 30 s, Round robin) que apunta a dos regiones etiquetadas `eu`, eu-west y eu-central. Cada una tiene un AWS ALB, 2 Go App Servers, un Stream Apache Kafka con 128 particiones, un Procesador de streams Apache Flink con 32 instancias (la Concurrencia por instancia por defecto de 4), un Almacén NoSQL DynamoDB y una Tabla lakehouse Apache Iceberg (Particionado Por día, Compactación Cada hora). El DynamoDB de eu-west está conectado con Replicar al de eu-central. Una región etiquetada `us` tiene un AWS ALB, 2 Go App Servers y un Almacén de datos Snowflake con 4 instancias para el panel; el Cliente llama a ese ALB directamente.
