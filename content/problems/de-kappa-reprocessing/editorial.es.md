# Solución comentada

## Adónde va la carga

Cada evento, en vivo o reprocesado, ocupa un hueco de Flink unos 15 ms: unos 5 ms de proceso y después la escritura en PostgreSQL y los saltos de red alrededor. Así que un hueco hace unos 67 eventos por segundo, y una tarjeta de Procesador de streams con 2 instancias de 4 huecos hace unos 540. Es la única etapa que importa: los Servidores de aplicaciones, Kafka y PostgreSQL están ocupados menos del 10 % en todas las pruebas.

La hora normal pide 600 eventos por segundo (300 en vivo, 300 reprocesados). Las pruebas ocultas cambian eso de dos maneras:

- El pico de la tarde con el reproceso acelerado envía 750 eventos en vivo y 1800 reprocesados por segundo: 2550 en total.
- Perder una tarjeta de Procesador de streams quita unos 540 eventos por segundo de proceso, durante el resto de la prueba.

La pregunta es quién lo paga. Un evento en vivo se mide por finalización, así que cada segundo que espera es un segundo de retraso en los paneles. Un evento reprocesado se mide por respuesta: en cuanto su Stream lo tiene, puede esperar lo que haga falta. Un Stream es FIFO, y los eventos reprocesados no llevan ninguna marca que permita a "Primero lo urgente (prioridad)" poner delante los en vivo: en un topic de Kafka solo son offsets más antiguos. Así que la única forma de sacar los eventos en vivo de la cola del reproceso es darle al reproceso una cola, y un procesador, propios.

## El diseño de referencia

Un AWS ALB envía los eventos a 2 Servidores de aplicaciones Go, que los publican en un Stream Apache Kafka con 16 Particiones y una Retención de 720 horas (30 días). Dos Procesadores de streams Apache Flink, cada uno con 2 instancias, una Concurrencia por instancia de 4 y 200 Registros por segundo por hueco, lo consumen: es el grupo de consumidores en vivo, y solo lee eventos nuevos. Escriben en PostgreSQL.

El reproceso es un segundo pipeline. El Planificador Airflow, que representa al reproceso leyendo el mes de nuevo, publica en su propio Stream Kafka con 8 Particiones, y una tercera tarjeta Flink con 2 instancias ejecuta sobre él el código corregido y escribe en el mismo PostgreSQL. En Kafka real es un segundo grupo de consumidores sobre el mismo topic, que empieza en los offsets de hace 30 días; la segunda tarjeta Stream es la forma en que el simulador le da sus propios offsets y su propio límite de particiones.

Cuesta unos 1261 $ al mes: 420 $ las tres tarjetas Flink, 350 $ PostgreSQL, 306 $ los dos Streams Kafka, 140 $ los Servidores de aplicaciones, 25 $ el ALB y 20 $ Airflow. En la hora normal las dos tarjetas Flink en vivo están ocupadas en torno al 28 % cada una, la del reproceso en torno al 54 %, y el p95 es de unos 37 ms.

"El pico de la tarde, con el reproceso acelerado a 6 veces su ritmo" envía 750 eventos en vivo por segundo al grupo de consumidores en vivo, que está ocupado en torno al 68 %, y 1800 eventos reprocesados por segundo a la tarjeta del reproceso, que puede con unos 540. La tarjeta del reproceso está al 100 % y su Stream se llena a unos 1260 eventos por segundo, hasta unos 75 000 al final de la prueba. Nada de eso llega a los eventos en vivo: el p95 se queda en unos 37 ms, sin errores y sin nada perdido. Acelerar el reproceso solo alargó la cola del propio reproceso.

"Se pierde un procesador de streams durante el reproceso" mata la primera tarjeta de Procesador de streams a los 20 s, para siempre. En la referencia es una de las dos tarjetas en vivo. La otra lleva sola los 300 eventos en vivo por segundo, ocupada en torno al 56 %, y el p95 se queda en unos 37 ms. Los pocos eventos que tenía la tarjeta muerta vuelven tras el Tiempo de sesión del Stream (45 s) y van a la superviviente; son demasiado pocos para mover el p95. Si la tarjeta perdida hubiera sido la del reproceso, el reproceso se pararía y esperaría en su Stream, y los eventos en vivo no lo notarían. En los dos casos la pérdida se queda en su lado.

Otros diseños que pasan, y algunos que no:

- Un solo grupo de consumidores para todo, dimensionado para el peor caso: dos tarjetas Flink con 5 instancias cada una y 40 Particiones superan las tres pruebas por unos 1388 $, un 10 % más que la referencia. Pasan solo porque la prueba se queda en 6 veces el ritmo: un reproceso que lee tan rápido como puede no tiene ese límite, y a un grupo compartido nunca le basta.
- Lo mismo con 4 instancias por tarjeta (1248 $) supera la hora normal y la pérdida del procesador, pero no el reproceso acelerado (p95 de unos 8,6 s).
- El pipeline de reproceso aparte, pero con una sola tarjeta Flink en vivo de 4 instancias en lugar de dos de 2, cuesta lo mismo que la referencia y falla la pérdida del procesador con un p95 de unos 66 s: la tarjeta perdida era la única en vivo.
- Una tarjeta de reproceso con 1 instancia sigue pasando (unos 1191 $). El reproceso solo tarda el doble; las pruebas no juzgan cuánto tarda.

Este problema no tiene reto de coste: el diseño más barato que encontró la búsqueda y que supera todas las pruebas cuesta unos 1121 $, el 89 % de la referencia.

## Por qué falla el primer intento

El primer intento habitual rebobina el job en vivo: un Stream Kafka con 16 Particiones (Retención 720 horas), dos tarjetas Flink con 2 instancias cada una, y el Planificador publicando el reproceso en el mismo Stream, así que un solo grupo de consumidores hace los dos trabajos. Cuesta unos 968 $ y supera la hora normal: 600 eventos por segundo sobre unos 1080 por segundo de proceso, cada tarjeta ocupada en torno al 54 %, con un p95 de unos 37 ms.

Con el pico de la tarde y el reproceso acelerado llegan 2550 eventos por segundo y se procesan unos 1080. Las dos tarjetas están al 100 %, y cada evento en vivo nuevo se pone al final de un montón que crece unos 1470 eventos por segundo. El p95 de cada segundo pasa de 5 s a unos 8 s de empezar la prueba, y el p95 de la prueba es de unos 49,3 s.

Cuando se pierde una tarjeta Flink, la superviviente recibe los 600 eventos por segundo y puede con unos 540. El montón crece más despacio, unos 60 por segundo, pero no para: el p95 de cada segundo pasa de 5 s hacia el segundo 84 de la prueba, y el p95 de la prueba es de unos 7,8 s. En las dos pruebas nada falla y nada se pierde; todos los eventos siguen en el Stream. Los paneles simplemente van con retraso.

Activar "Primero lo urgente (prioridad)" no cambia nada aquí: ningún evento está marcado como masivo, porque un evento reprocesado es el mismo evento leído otra vez. Esa es la diferencia con el problema del relleno (DE 12), cuyos lotes históricos entraban por la API y se podían marcar.

La idea es esta: un reproceso no comparte nada que no tenga que compartir. Ejecútalo como su propio grupo de consumidores en su propio procesador, para que su velocidad y sus fallos sean solo suyos, y deja una reserva en el lado en vivo, porque un pipeline en vivo que pierde un procesador tiene que seguir llevando el pico en vivo.

## Lo que el simulador deja fuera

Los grupos de consumidores de Kafka no se modelan: una tarjeta Stream entrega cada evento a uno de sus consumidores. La segunda tarjeta Stream de la referencia representa el mismo topic leído por un segundo grupo de consumidores, con sus propios offsets; en un clúster real no costaría brokers de más. La retención tampoco se simula: la Retención de 720 horas está para mostrar los 30 días que necesita el reproceso. El reproceso es un ritmo de llegada fijo desde la tarjeta Planificador. Un reproceso real lee tan rápido como puede su consumidor y nunca se amontona; aquí los eventos que no puede procesar esperan en su Stream, que guarda hasta 100 000 (pasado eso, se rechazan las publicaciones). No se juzga cuánto tarda el reproceso, ni escribir las filas corregidas en una versión nueva de la tabla y cambiar los paneles a ella; los dos pipelines escriben en una sola tarjeta PostgreSQL, que tiene margen de sobra. El simulador dejaría que más huecos por instancia procesaran más, cosa que un job limitado por la CPU no puede hacer; por eso el enunciado fija la Concurrencia por instancia y los Registros por segundo por hueco. Una tarjeta Flink que muere vuelve a procesar sus registros desde el último checkpoint, y el simulador cuenta esos duplicados (unos 3000 en la prueba de la pérdida del procesador de la referencia), pero este problema no los juzga (mira DE 11, Sesionización de clics). Los reequilibrios del grupo de consumidores, que pausan un grupo compartido real cuando se va un miembro, no se simulan. El reproceso entero dura días; cada prueba ve uno o dos minutos de él.

## Para probar

- En la referencia, conecta el Planificador al Stream en vivo en lugar del Stream del reproceso, y ejecuta el reproceso acelerado. Mira cómo los eventos en vivo esperan detrás del mes.
- Cambia las dos tarjetas Flink en vivo por una de 4 instancias y ejecuta la pérdida del procesador. La misma capacidad, el mismo coste, y nada cuando se cae.
- Dale 1 instancia a la tarjeta del reproceso y mira su Stream durante el reproceso acelerado. El procesador del reproceso decide cuánto tarda el reproceso, y nada más lo decide.
