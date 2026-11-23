# Solución comentada

## Adónde va la carga

Con 1500 publicaciones por segundo, cada publicación pasa una vez por el clasificador. Una publicación típica tiene unos 40 tokens: Triton tarda unos 5 ms en empezar y lee el texto a unos 5000 tokens por segundo, así que una comprobación tarda unos 5 + 8 = 13 ms (unos 23 ms para una publicación larga de 90 tokens). Así, 1500 × 0,013 s mantienen unas 20 publicaciones en curso. Una instancia de GPU tiene 64 Huecos de lote, así que en un día normal está ocupada en torno a un tercio.

Las pruebas ocultas cambian eso. «Una publicación se hace viral» multiplica el tráfico por 5 a partir del segundo 15, durante 30 segundos: 7500 publicaciones por segundo, unas 98 en curso, más que los 64 huecos de una instancia. «Se pierde un servidor del modelo» mata la primera tarjeta de Inferencia de modelos en el segundo 20, y no vuelve. Cada publicación enviada a esa tarjeta no tiene otro sitio adonde ir.

## El diseño de referencia

Un AWS ALB envía las publicaciones a 4 Go App Servers. Estos llaman a dos tarjetas de Inferencia de modelos NVIDIA Triton, cada una con 2 Instancias, el Tipo de modelo en Clasificador y los 64 Huecos de lote por GPU por defecto, y reparten las llamadas entre las dos. Cuesta unos 3241 $ al mes: 1468 $ cada tarjeta Triton, 280 $ los App Servers y 25 $ el ALB. Las GPU son la mayor parte de la factura. Un día normal tiene un p95 de unos 36 ms sin errores, y cada tarjeta Triton está ocupada en torno al 8 %.

Las 2 instancias de cada tarjeta no están ahí por el día normal. Cada tarjeta tiene 128 huecos, suficientes para llevar sola todo el pico viral (unas 98 publicaciones en curso). Durante «Una publicación se hace viral», las dos tarjetas se lo reparten, y el p95 se queda en unos 37 ms sin errores.

«Se pierde un servidor del modelo» admite un 3 % de errores y un 97 % de disponibilidad. El diseño falla en un 1,7 % de las publicaciones, todas en los 2 segundos más o menos que tardan las comprobaciones de salud en notar la tarjeta caída: hasta entonces, la mitad de las llamadas siguen yendo a ella. Después, todas las llamadas van a la otra tarjeta, que lleva toda la carga con un tercio de sus huecos, y el p95 se mantiene en unos 36 ms.

Para el reto de coste, un diseño de prueba se queda en el 53 % del coste de la referencia, por debajo del objetivo del 55 %. Mantiene dos tarjetas Triton, con una instancia cada una, y 3 App Servers. Juntas, las dos tarjetas de una instancia siguen teniendo 128 huecos para el pico, y cualquiera de ellas lleva sola un día normal; lo que pierde es el margen para perder una tarjeta durante un pico.

## Por qué falla el primer intento

Un primer intento habitual es la misma entrada y los mismos App Servers con una sola tarjeta NVIDIA Triton de 1 instancia. En un día normal está ocupada en torno a un tercio y parece sobredimensionada. Cuesta unos 1039 $ y supera «Un día normal» con un p95 de unos 36 ms.

Cuando una publicación se hace viral, unas 98 publicaciones quieren un hueco a la vez y la instancia tiene 64. Las demás esperan en la cola, la cola se llena y se descartan publicaciones: un 32 % de errores en toda la ejecución (en torno al 40 % de cada segundo del pico), con un p95 de unos 519 ms. Una segunda instancia en esa tarjeta llevaría el pico.

Cuando se pierde el servidor del modelo, no hay un segundo. Falla cada publicación a partir del segundo 20: un 67 % de errores en toda la ejecución. Más instancias en la misma tarjeta no ayudarían, porque cae la tarjeta entera.

La idea es esta: un nivel de GPU que parece ocioso en un día medio está dimensionado para la media. Dimensiónalo para el pico, y repártelo entre al menos dos servidores de modelo para que perder uno sea un tropiezo breve.

## Lo que el simulador deja fuera

No se simula la precisión del clasificador: una publicación cuenta como comprobada tanto si la etiqueta acierta como si no, y aquí nada mide los falsos positivos ni las publicaciones que se escapan. No hay cola de revisión para las publicaciones que marca el clasificador, ni un respaldo que retenga una publicación para más tarde. En el simulador, un grupo de autoescalado no puede contener una tarjeta de Inferencia de modelos, así que aquí el nivel del modelo tiene un tamaño fijo; además, un nodo de GPU nuevo de verdad tiene que arrancar y cargar el modelo antes de servir, y eso no se simula. Tampoco se simulan las imágenes, el vídeo ni el almacenamiento de la publicación.

## Para probar

- Dale 2 instancias a la tarjeta Triton del primer intento y ejecuta «Una publicación se hace viral». Después vuelve a ejecutar «Se pierde un servidor del modelo».
- Baja las dos tarjetas Triton a 1 instancia y ejecuta el pico. Mira lo ocupados que llegan a estar los huecos.
- Pon los Huecos de lote por GPU del primer intento en 32 y ejecuta «Un día normal». ¿Cuánto le falta para llenarse?
