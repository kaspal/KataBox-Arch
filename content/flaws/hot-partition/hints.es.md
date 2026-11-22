## 1

El almacén está casi ocioso y aun así las peticiones esperan. Mira cómo se reparte la carga, no cuánta hay.

## 2

Cada lectura del mismo perfil va a la misma partición. ¿Qué quita al almacén las lecturas de una sola clave?

## 3

Pon una caché Redis entre la API y DynamoDB (cache-aside).
