## 1

Cuenta los puntos por segundo en una tormenta, y cuánto espera cada uno a su puntuación. Quien llama al clasificador ocupa un hueco durante toda esa espera. Después cuenta las series que lleva cada nodo de la base de datos, y multiplica por 8.

## 2

Si el Servidor de aplicaciones puntúa cada punto en línea, sus huecos quedan ocupados mientras trabaja el modelo, así que una tormenta los llena y los puntos se rechazan. Pon un stream después del Servidor de aplicaciones y deja que un consumidor llame al clasificador y escriba en la base de datos: el Servidor de aplicaciones solo publica. El consumidor pasa casi todo el tiempo esperando, así que dale hilos que salgan baratos mientras esperan. Después dimensiona el clasificador para la tormenta, no para el día normal: una GPU que se queda corta en una tormenta retrasa todas las alertas. Para las series, cada instancia más de la base de datos lleva una parte menor; planifícalas para que 8 veces las series sigan por debajo del límite de cada nodo.

## 3

Envía los puntos desde el Servidor de aplicaciones Go (2 instancias) a un Stream Apache Kafka con 384 particiones. Un Worker Celery (6 instancias, Concurrencia 64 por instancia, Tiempo de servicio 1 ms) lo consume, llama a NVIDIA Triton con 4 instancias y escribe en una Base de datos de series temporales InfluxDB con 3 instancias de 20 000 puntos por segundo cada una. Mantén los sensores conectados a un Broker IoT EMQX con las 2 instancias por defecto.
