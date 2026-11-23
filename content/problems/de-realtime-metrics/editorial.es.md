# Solución comentada

## Adónde va la carga

5000 eventos por segundo llegan a los Servidores de aplicaciones, que solo los publican en el Stream: esa parte es fácil. El trabajo está en el Procesador de streams, y tiene dos límites distintos.

El primero es el tiempo. Una ventana fija de 30 segundos con una marca de agua de 5 s retrasa cada resultado hasta 35 s, unos 33 s en el p95, hagas lo que hagas. Eso deja unos 10 s del objetivo de 45 s para las colas. Un evento que llega al procesador después de que su ventana se haya cerrado es tardío, salvo que el Retraso permitido aún lo cubra. Los eventos web (0,3 s de mediana, 3 s en el p99) casi nunca son tardíos. Los móviles (5 s de mediana, 120 s en el p99) lo son a menudo: un evento al final de su ventana solo tiene los 5 s de la marca de agua antes de que la ventana se cierre.

El segundo es el rendimiento. Un hueco procesa aquí unos 800 eventos por segundo, así que 5000 eventos por segundo necesitan unos 6,3 huecos ocupados, y los 15 000 del lanzamiento, unos 19. Las Particiones del Stream limitan cuántos huecos pueden trabajar.

Las pruebas ocultas atacan cada límite por turno. "Los móviles recuperan la conexión" multiplica por 20 los eventos móviles: 2000 eventos móviles por segundo, cerca del 29 % de todos los eventos en lugar del 2 %. "Día de lanzamiento" triplica todo el tráfico durante 40 segundos, desde el segundo 20 de una ejecución de 90 s.

## El diseño de referencia

Un AWS ALB envía el tráfico a 4 Servidores de aplicaciones Go. Estos publican los eventos en un Stream Apache Kafka con 32 Particiones y leen el panel de una Base de datos de series temporales TimescaleDB. Un Procesador de streams Apache Flink de 8 instancias con una Concurrencia por instancia de 4 (32 huecos, uno por partición) consume el Stream: Ventana Fija (tumbling), Duración de la ventana 30 s, Retraso de la marca de agua 5 s, Retraso permitido 180 s. Está conectado a la Base de datos de series temporales con una conexión Query. Cuesta unos 1278 $ al mes: 560 $ el Procesador de streams, 280 $ los Servidores de aplicaciones, 260 $ TimescaleDB, 153 $ Kafka y 25 $ el ALB.

En un día normal el procesador está ocupado en torno al 19 % y el tiempo hasta un evento contado es de unos 33,2 s en el p95, sin errores. Menos del 0,03 % de los eventos en ventanas son tardíos.

El Retraso permitido se dimensiona con la distribución del retraso: 180 s supera el p99 móvil de 120 s, incluso para un evento justo al final de su ventana (5 s + 180 s). Un evento móvil tardío actualiza al momento su ventana cerrada en lugar de quedarse fuera. Así, cuando los móviles recuperan la conexión, la tasa de tardíos se queda por debajo del 0,03 % y el p95 se mantiene en unos 33,2 s.

La marca de agua se queda en 5 s. Una marca de agua más larga también atraparía más rezagados, pero retrasa cada resultado, no solo los tardíos: con una marca de agua de 120 s, cada recuento del panel llegaría con más de dos minutos de retraso.

En el día de lanzamiento, 15 000 eventos por segundo llegan a 32 huecos que pueden con unos 25 600 por segundo: ocupados en torno al 59 % en el pico. No hay errores, y el p95 se mantiene en unos 33,2 s.

Para el reto de coste, un diseño de prueba se queda en el 67 % del coste de la referencia, por debajo del objetivo del 70 %. Mantiene las 32 Particiones y los mismos ajustes de ventana, con 2 Servidores de aplicaciones y 4 instancias de Flink (16 huecos): en el pico del lanzamiento trabajan por encima de su capacidad, pero la cola que acumulan en 40 s es lo bastante pequeña para vaciarse sin errores y dentro del objetivo de 45 s.

## Por qué falla el primer intento

Un primer intento habitual usa el mismo borde y los mismos Servidores de aplicaciones, un Stream Kafka con 12 Particiones y un Procesador de streams Flink de 3 instancias con una Concurrencia por instancia de 4 (12 huecos), con la misma ventana fija de 30 s y la marca de agua de 5 s, pero con el Retraso permitido por defecto, 0. Cuesta unos 928 $ y supera "Un día normal": el procesador está ocupado en torno al 52 %, el p95 es de unos 33,2 s y entre el 0,1 % y el 0,2 % de los eventos son tardíos, todos móviles.

Cuando los móviles recuperan la conexión, los eventos móviles pasan a ser cerca del 29 % del tráfico. Cada evento móvil que llega después del final de su ventana más 5 s se queda fuera de los recuentos: entre el 2 % y el 4 % de todos los eventos en ventanas son tardíos, por encima del límite del 1 %, y la prueba falla por incumplir el SLO. Al procesador le sobra capacidad; simplemente, las ventanas se cierran demasiado pronto para estos eventos.

En el día de lanzamiento, 15 000 eventos por segundo llegan a 12 huecos que pueden con unos 9600. A partir de unos 18 s de pico, la cola del procesador se desborda, y cerca del 36 % de los eventos de cada segundo se rechazan hasta que acaba el pico: un 13,6 % de errores en la ejecución. Añadir solo instancias no ayudaría: con 12 Particiones, los huecos de más se quedarían parados.

La idea es esta: una ventana necesita dos números, no uno. La marca de agua fija lo frescos que son los resultados; el retraso permitido fija cuántos eventos tardíos siguen contando. Dimensiona el retraso con lo tarde que llegan de verdad tus clientes más lentos, y dimensiona huecos y particiones juntos para el pico.

## Lo que el simulador deja fuera

Las escrituras del procesador en la Base de datos de series temporales no se envían: un evento cuenta como hecho cuando el resultado de su ventana sale del procesador (las escrituras reales son unas pocas filas por métrica y ventana). Una actualización tardía no se factura ni se cronometra como una escritura más. Mantener las ventanas abiertas durante el retraso permitido cuesta memoria en un procesador real; aquí el estado de la ventana no crece por ello. El tiempo de evento sale de un retraso sorteado para cada evento, no de los relojes de los dispositivos, así que los relojes de teléfono mal puestos no se simulan. Las lecturas del panel son consultas simples, y los recuentos en sí no se comprueban: solo se cuentan los eventos tardíos.

## Para probar

- Pon el Retraso permitido en 60 s y ejecuta "Los móviles recuperan la conexión". Compara la tasa de tardíos con 0 s y con 180 s.
- Sube en su lugar el Retraso de la marca de agua a 30 s, con un Retraso permitido de 0. Mira la tasa de tardíos y el p95.
- Deja 8 instancias de Flink pero pon el Stream en 12 Particiones, y ejecuta "Día de lanzamiento".
