## 1

De 200 000 prefijos, ¿cuántos distintos escribe la gente la mayor parte del tiempo, y en qué parte del índice caen?

## 2

Cachea las respuestas de los prefijos populares delante del índice. Añadir shards reparte prefijos distintos, pero un prefijo caliente siempre cae en el mismo shard.

## 3

Pon una Cache Redis con un TTL de alrededor de una hora entre los App Servers y el índice de Search, y deja el índice detrás con unos pocos shards. Unos pocos App Servers bastan cuando la mayoría de las pulsaciones terminan en la caché.
