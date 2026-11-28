# El panel de las 9 de la mañana

## El producto

Tu empresa tiene un panel que abre todo el mundo: las ventas de ayer, los pedidos de hoy, el stock por almacén. Sus números salen de un Almacén de datos (Snowflake) que el equipo de datos carga cada hora, así que un número hasta diez minutos más viejo que el almacén sirve. Casi todo el día el panel está tranquilo. Pero a las 9 de la mañana toda la empresa se sienta, lo abre a la vez, y el mismo puñado de consultas llega por cientos.

## Lo que debes cumplir

| Objetivo | Valor |
| --- | --- |
| Tráfico pico | 20 cargas de recuadro/s en una hora normal |
| Latencia p95 | 2000 ms o menos |
| Disponibilidad | 99,9 % o más |
| Errores | 1 % o menos |
| Presupuesto | 8500 $ al mes |

## El tráfico

- **Cargar un recuadro del panel** (100 %). La petición puede pasar por un balanceador de carga y llega a tus Servidores de aplicaciones. Estos pueden leer el recuadro de una caché (cache-aside); en un fallo de caché, o sin caché, el Servidor de aplicaciones ejecuta la consulta del recuadro en el Almacén de datos y la espera. Hay 400 recuadros distintos (gráficos por filtros), y unos pocos se llevan la mayoría de las cargas (una popularidad Zipf de 1,0).

Cada consulta de un recuadro recorre unos 0,5 GB de las tablas propias del almacén. Con los valores por defecto de la tarjeta Snowflake (Sobrecarga por consulta 300 ms, Velocidad de escaneo por consulta 1 GB/s) son unos 0,8 s por consulta. Un clúster ejecuta 8 consultas a la vez (Concurrencia por instancia) y encola el resto, pero las consultas que se ejecutan en él comparten su 1 GB/s de escaneo: ocho escaneos a la vez reciben cada uno una octava parte. Cada clúster más (Instancias) añade otros 8 huecos, otro 1 GB/s y su propio precio. Alrededor del 60 % de las consultas repiten una consulta anterior cuyos datos no han cambiado desde entonces, y la Caché de resultados del almacén las responde en unos 30 ms. La Caché de resultados viene activada por defecto.

Snowflake cobra un clúster solo mientras funciona: los segundos que está ocupado, cada uno alargado por Suspensión automática tras (60 s). El cliente se rinde a los 5 s.

## Las pruebas

Hay una prueba visible. Puedes ejecutarla tantas veces como quieras:

- **Una hora normal**: el tráfico de arriba.

Hay dos pruebas más ocultas, y solo ves sus nombres: **Las 9 de la mañana: cachés vacías y luego diez veces el tráfico durante 40 s** y **El almacén de datos se ralentiza: las consultas tardan 3 s más durante 40 s**. Tu diseño tiene que pasar las tres. El simulador ejecuta pruebas cortas, así que la avalancha de la mañana se reduce a 40 segundos; una real dura más, y eso solo empeora la cola.
