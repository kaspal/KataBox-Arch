## 1

Una ventana no puede esperar para siempre a sus rezagados, pero un teléfono puede llegar dos minutos tarde. ¿Qué pasa con un evento que llega después de que su ventana se haya cerrado? Y cuando llegan tres veces más eventos, ¿cuántos puede atender el procesador a la vez?

## 2

Dimensiona el Retraso permitido con la distribución del retraso, no con la marca de agua: los eventos móviles llegan hasta 120 s tarde en el p99, así que deja que una ventana cerrada acepte actualizaciones durante más tiempo que eso. Mantén corto el Retraso de la marca de agua, porque cada segundo suyo se suma a cada resultado. Para el rendimiento, cuenta huecos: unos 800 eventos por segundo cada uno, y las Particiones del Stream limitan cuántos pueden trabajar.

## 3

Pon un AWS ALB delante de 4 Servidores de aplicaciones Go que publican en un Stream Apache Kafka con 32 Particiones y leen el panel de una Base de datos de series temporales TimescaleDB. Consume el Stream con un Procesador de streams Apache Flink de 8 instancias con una Concurrencia por instancia de 4: Ventana Fija (tumbling), Duración de la ventana 30 s, Retraso de la marca de agua 5 s, Retraso permitido 180 s. Conecta el procesador a la Base de datos de series temporales con una conexión Query.
