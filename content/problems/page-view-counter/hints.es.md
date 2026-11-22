## 1

Los Workers que cuentan las visitas las toman de un stream. ¿Cuántas visitas puede entregar ese stream a la vez, y cambia esa cifra si añades Workers?

## 2

En un stream, cada partición entrega un mensaje cada vez, así que el número de particiones limita cuántas visitas se cuentan a la vez. Contar una visita lleva unos 8 a 10 ms. Calcula cuántas visitas por segundo pueden contar tus particiones y deja margen para el doble de tráfico o para Workers más lentos.

## 3

Dale al stream Kafka unas 48 particiones y dales a los Counters al menos tantos huecos como particiones: Celery tiene 8 por instancia por defecto, así que 8 instancias dan 64. Pon una Cache Redis delante de la base de datos para el panel, con un TTL corto, por ejemplo de 10 segundos.
