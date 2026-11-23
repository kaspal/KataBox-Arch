## 1

Compara lo que escribe por segundo un nodo de la Base de datos de series temporales con lo que llega, en un día normal y cuando una zona entera sube lo que guardó sin conexión. Luego cuenta las series que lleva cada nodo, y multiplícalas por 10.

## 2

Una base de datos llena rechaza las escrituras; un stream delante las guarda y se las entrega al ritmo que la base puede aguantar. Los workers que leen el stream marcan ese ritmo, así que dimensiónalos para la base de datos, no para la ráfaga. Para las series, más instancias ayudan dos veces: cada una suma sus puntos por segundo, y cada una lleva una parte menor de las series. Planifica las instancias para que diez veces las series sigan por debajo del límite de cada nodo.

## 3

Envía las lecturas desde el Servidor de aplicaciones Go (2 instancias) a un Stream Apache Kafka con 192 particiones. Seis Workers Celery lo consumen (concurrencia 32 por instancia, tiempo de servicio 2 ms) y escriben en una Base de datos de series temporales InfluxDB con 3 instancias de 4000 puntos por segundo cada una. Mantén los contadores conectados a un Broker IoT EMQX con las 2 instancias por defecto.
