# Solución comentada

## Adónde va la carga

Con 4000 rps, el 70 % son mosaicos: 2800 por segundo. Se pueden cachear, y unos pocos mosaicos con mucho movimiento reciben la mayoría de las peticiones, así que una CDN puede responder casi todas y el almacenamiento de objetos solo ve los fallos de caché. La CDN cobra por petición, así que 2800 mosaicos por segundo es también donde se va la mayor parte del dinero.

El otro 30 % son rutas: 1200 por segundo. Cada una recorre 6 saltos del grafo de carreteras con 3 carreteras por cruce, unas 1100 relaciones, así que la base de datos de grafos visita unos 1,3 millones de relaciones por segundo. Después cada ruta llama al feed de tráfico. A 40 ms por llamada, 1200 × 0,04 s mantienen unas 48 llamadas en curso, cada una ocupando un hueco del router. Eso es barato mientras el feed responde. El peligro es un feed que deja de responder: una ruta que espera alrededor de 1 s a un feed caído ocupa su hueco 25 veces más, y 1200 rutas por segundo necesitarían unos 1200 huecos solo para esperar.

## El diseño de referencia

El Cliente carga los mosaicos de una CDN CloudFront con un TTL de 86 400 s (un día), delante de Amazon S3. Las rutas pasan por un AWS ALB hacia Servidores de aplicaciones Go, los routers, con 5 instancias, el Tiempo límite de llamada en 150 ms y el Cortocircuito con errores al 50 %. Consultan una Base de datos de grafos Neo4j con 3 instancias. Cuesta unos 8418 $ al mes: 7370 $ de CloudFront, que cobra por petición con 2800 mosaicos por segundo, 552 $ de Neo4j, 350 $ de los routers, 121 $ de S3 y 25 $ del ALB.

«Una tarde normal» tiene un p95 de unos 96 ms sin errores, y los routers están ocupados en torno a un 23 %.

«Hora punta» multiplica el tráfico por 2,5 a partir del segundo 15, durante 30 segundos. La CDN absorbe los mosaicos de más y los routers tienen sitio para las rutas de más: el p95 sigue en unos 96 ms, sin errores.

«Cae el feed de tráfico» detiene el feed a partir del segundo 20, durante 40 segundos, en una ejecución de 90 segundos. Las primeras llamadas agotan su tiempo a los 150 ms, y esas rutas recurren a las velocidades típicas. En pocos segundos falla la mayoría de las llamadas, el cortocircuito se abre y los routers dejan de llamar al feed: fallan al instante. Las rutas vuelven más rápido de lo habitual, con horas de llegada de velocidades típicas. El p95 queda en unos 73 ms, sin errores.

Este problema no tiene reto de coste. El mejor diseño más barato que se encontró seguía costando cerca del 98 % de la referencia: la factura de la CDN domina, y todos los diseños tienen que servir los mismos mosaicos.

## Por qué falla el primer intento

Un primer intento habitual es el mismo diseño con 3 routers, sin tiempo límite de llamada y sin cortocircuito. Cuesta unos 8278 $ al mes. Sale Aceptado en «Una tarde normal», con los routers ocupados en torno a un 39 %, y supera «Hora punta» con un p95 de unos 102 ms.

Cuando cae el feed, falla en torno al 12,3 % de las peticiones. Cada ruta sigue esperando al feed caído, alrededor de 1 s hasta que la llamada falla, y ocupa un hueco del router todo ese tiempo. Los huecos se acaban, las rutas nuevas hacen cola detrás y muchas fallan. El feed era opcional sobre el papel, pero los routers lo trataban como obligatorio.

La idea es esta: una dependencia que solo es un extra tiene que ser opcional en la práctica. Un tiempo límite corto, un cortocircuito y un plan alternativo (las velocidades típicas) convierten su caída en respuestas algo peores en vez de en una caída tuya.

## Lo que el simulador deja fuera

No se simula el algoritmo de búsqueda de rutas: cada ruta visita un número fijo de relaciones. Tampoco las actualizaciones de los datos del mapa ni el renderizado de los mosaicos. No se puntúa la precisión de la hora de llegada, así que una hora de velocidades típicas cuenta igual que una de tráfico en vivo. La CDN se cobra solo por petición, sin coste por ancho de banda.

## Para probar

- Quita la CDN y envía los mosaicos directamente a Amazon S3. Compara el coste y el p95 con la referencia.
- Deja el Tiempo límite de llamada en 150 ms pero pon el Cortocircuito con errores a 0 (apagado), y ejecuta «Cae el feed de tráfico». ¿Cuánto salva el tiempo límite por sí solo, y qué añade el cortocircuito?
- Baja los routers a 3 instancias con el tiempo límite y el cortocircuito activados, y ejecuta las tres pruebas. ¿Lo que salva la caída es la capacidad de más o el cortocircuito?
