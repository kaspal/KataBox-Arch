# Documentos colaborativos

## El producto

Llevas un editor de documentos en el que varias personas pueden trabajar a la vez sobre la misma página. Mientras un documento está abierto, el editor mantiene un WebSocket con uno de tus servidores de sincronización, y cada pulsación sale como una pequeña edición. Dos personas en el mismo documento pueden estar conectadas a servidores de sincronización distintos, y aun así cada una tiene que ver lo que escribe la otra casi en el momento. De vez en cuando, el editor guarda además una instantánea completa del documento, para que no se pierda nada si todos lo cierran. Cuando el cursor del otro se congela o sus palabras llegan con segundos de retraso, escribir juntos deja de funcionar.

## Lo que debes cumplir

| Objetivo | Valor |
| --- | --- |
| Tráfico pico | 6000 mensajes/s |
| Latencia p95 hasta completar | 150 ms o menos |
| Disponibilidad | 99,9 % o más |
| Presupuesto | 11 000 $ al mes |

## El tráfico

Los dos tipos de mensaje llegan por un WebSocket, a través de un componente de borde, a un servidor de sincronización, que es un App Server.

- **Escribir una edición** (90 %). El servidor de sincronización publica la edición en un broker. Todos los servidores de sincronización que tengan editores de ese documento deben recibirla, para hacérsela llegar a ellos. Una edición termina cuando ha llegado a los servidores de sincronización, no cuando el primero la acepta, y su latencia se mide hasta ese punto.
- **Guardar una instantánea** (10 %). El servidor de sincronización escribe una copia del documento, de unos 16 KB, en una base de datos.

En el pico hay unos 100 000 documentos abiertos. Planifica el coste del broker con el mismo cuidado que su velocidad: una tarjeta Pub/Sub cobra por cada mensaje que transporta, y las ediciones son la mayor parte del tráfico.

## Las pruebas

Hay una prueba visible. Puedes ejecutarla tantas veces como quieras:

- **Una jornada normal**: el tráfico descrito arriba, en su pico.

Hay dos pruebas ocultas más, de las que solo ves el nombre: **500 editores en un mismo documento** y **Se cae un servidor de sincronización**. Tu diseño tiene que superar las tres.
