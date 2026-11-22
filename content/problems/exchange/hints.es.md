## 1

Al sonar la campana de apertura, las órdenes llegan varias veces más deprisa que durante la sesión, y un matcher puede caerse en cualquier momento. ¿Qué limita cuántas órdenes casa tu bolsa a la vez, y qué sigue casando cuando se cae un matcher?

## 2

Un stream entrega un mensaje por partición cada vez, así que el número de particiones limita cuántas órdenes se casan a la vez; dimensiónalo, igual que la Order gateway, para cuatro veces el flujo normal. Después reparte los matchers en más de una tarjeta. Si una sola tarjeta tiene todos los matchers, una sola caída detiene todo el casamiento.

## 3

Usa 6 instancias Go en la pasarela detrás de un Network Load Balancer, un stream Kafka con 128 particiones, y dos tarjetas de matchers Celery con 12 instancias cada una: cualquiera de las dos aguanta ella sola el triple de una sesión normal. Los matchers escriben en un libro de órdenes Redis con un TTL largo.
