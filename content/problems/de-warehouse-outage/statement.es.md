# El almacén de datos se cae

## El producto

Tus apps móvil y web envían sus eventos (pantallas vistas, botones pulsados, compras) a un grupo de recolectores de eventos. Cada recolector junta unos 500 eventos y los reenvía como un lote, y cada lote tiene que acabar en el almacén de datos, Snowflake, donde los analistas lo consultan. En una hora con mucho movimiento son 30 lotes por segundo; en el pico de la tarde, 45.

Snowflake no siempre está disponible. Una ventana de mantenimiento, un despliegue fallido del proceso de carga o un incidente en la nube pueden dejarlo fuera de servicio durante un rato. Cuando eso pasa, no se puede perder ningún lote, y en cuanto Snowflake vuelve, el almacén tiene que ponerse al día rápido: un analista que abre un panel después del incidente debería ver datos frescos otra vez, no datos que siguen atascados muy por detrás.

En la vida real, caídas así duran decenas de minutos, y la recuperación una hora o así. Los percentiles del simulador llegan hasta un minuto más o menos, así que el problema está a escala: aquí el almacén cae durante **60 s**, y el atasco tiene que despacharse en **50 s** o menos desde que vuelve.

## Lo que debes cumplir

| Objetivo | Valor |
| --- | --- |
| Tráfico | 30 lotes/s en una hora con mucho movimiento, 45 en el pico de la tarde |
| Frescura en un día normal (p95, desde el recolector hasta que el lote está en el almacén) | 10 s o menos |
| Frescura en una prueba con caída (p95 de toda la prueba) | 60 s o menos |
| Recuperación tras una caída en el pico | desde 50 s después de que vuelva el almacén, en cada segundo, el 95 % de los lotes cargados esperó menos de 60 s |
| Disponibilidad | 99,9 % o más |
| Errores (lotes rechazados) | 0,1 % o menos |
| Lotes perdidos | 0 |
| Presupuesto | 11 000 $ al mes |

## El tráfico

- **Cargar un lote de eventos** (100 %). Un recolector envía el lote; puede pasar por un balanceador de carga y llega a tus App Servers. Desde ahí va al almacén de datos, o bien directamente (el App Server lo escribe y responde cuando ya está cargado), o bien a través de un Stream: el App Server publica el lote y responde enseguida, y un consumidor (un Conector de datos como Kafka Connect, un Worker o un Procesador de streams) lo toma del Stream y lo carga. Hay 1 000 000 de claves de lote distintas, todas más o menos igual de probables.

La latencia se mide hasta completar: desde que el recolector envía el lote hasta que está en el almacén.

Una carga en Snowflake tarda unos 300 ms de mediana, con una cola lenta. Una instancia (un clúster X-Small) ejecuta 8 cargas a la vez; deja la **Concurrencia por instancia en 8** y añade instancias para tener más cargas. Snowflake cobra una instancia por cada segundo que está ocupada, más su tiempo de Suspensión automática, así que un almacén que carga todo el día cuesta el mes entero.

Cuando Snowflake está caído, una carga falla al momento. Un Stream conserva cada registro hasta que se ha cargado: un consumidor cuya carga falla lo vuelve a intentar más tarde, esperando 1 s, luego 2, 4, 8, 16 y como mucho 32 s entre intentos, y la Retención del Stream (7 días por defecto) dura más que cualquier caída de este problema. Un Stream entrega un registro por partición cada vez.

## Las pruebas

Hay una prueba visible. Puedes ejecutarla tantas veces como quieras:

- **Un día normal**: 30 lotes por segundo durante 60 s.

Hay dos pruebas ocultas más, de las que solo ves el nombre: **El almacén de datos cae durante 60 s** y **El almacén de datos cae durante 60 s en el pico de la tarde: 1,5 veces los eventos**. Las dos duran 600 s, y la caída empieza a los 20 s, así que el límite de recuperación de arriba se aplica desde los 130 s. Tu diseño tiene que superar las tres.
