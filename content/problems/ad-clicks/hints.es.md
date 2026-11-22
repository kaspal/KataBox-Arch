## 1

Cada clic se convierte en una escritura en un contador. ¿Cuántas escrituras por segundo son, y qué pasa con esa cifra cuando los clics llegan varias veces más deprisa durante unos segundos?

## 2

Un incremento es muy poco trabajo. Un almacén que guarda los contadores en memoria lo hace en una fracción de milisegundo, mientras que una escritura en base de datos tarda unos milisegundos y la base de datos tiene un número fijo de huecos; un agregador que espera a una base de datos ocupada conserva su hueco todo ese tiempo. Cuenta también las particiones del stream: cada una entrega un clic cada vez, así que entre todas limitan lo deprisa que se cuentan los clics. Deja margen para cuatro veces el tráfico durante unos segundos.

## 3

Guarda los contadores en una Cache Redis con un TTL largo, por ejemplo un día (86 400 s), y lee también de ella los totales. Dale al stream Kafka unas 128 particiones y dales a los Aggregators otros tantos huecos: 16 instancias Celery con la concurrencia por defecto de 8.
