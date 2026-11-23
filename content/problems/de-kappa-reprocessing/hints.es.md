## 1

Rebobinar el job en vivo pone un mes de eventos antiguos delante de cada evento nuevo. ¿Cuánto espera un evento en vivo detrás de ellos, y qué pasa cuando se acelera el reproceso, o cuando desaparece uno de los procesadores que hacen los dos trabajos?

## 2

Dale al reproceso su propio grupo de consumidores: una segunda tarjeta de Procesador de streams que lee los eventos retenidos de su propio Stream (sus propios offsets) y escribe con el código corregido, mientras el job en vivo sigue leyendo solo eventos nuevos. Así nada de lo que haga el reproceso, ir más rápido o caerse, llega a los eventos en vivo. Dimensiona el lado en vivo para el pico de la tarde y deja uno de reserva: cuando se pierde una tarjeta de Procesador de streams en vivo, la otra tiene que llevar sola los eventos en vivo.

## 3

Pon un AWS ALB delante de 2 Servidores de aplicaciones Go que publican en un Stream Apache Kafka con 16 Particiones (Retención 720 h). Dos Procesadores de streams Apache Flink, cada uno con 2 instancias (Concurrencia por instancia 4, Registros por segundo por hueco 200), lo consumen y escriben en PostgreSQL. Conecta el Planificador Airflow a un segundo Stream Apache Kafka con 8 Particiones, consumido por una tercera tarjeta Flink con 2 instancias y los mismos valores, que escribe en el mismo PostgreSQL.
