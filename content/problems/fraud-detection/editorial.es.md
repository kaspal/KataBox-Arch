# Solución comentada

## Adónde va la carga

Con 3000 solicitudes por segundo, el 60 % son puntuaciones y el 40 %, eventos: 1800 puntuaciones y 1200 eventos por segundo. Cada puntuación espera unos 20 ms al modelo, así que 1800 × 0,02 s mantienen unas 36 puntuaciones en curso en el modelo, y otros tantos huecos de la API ocupados esperándolo. Es poco trabajo para dos GPU.

Las pruebas ocultas cambian esas cifras. En Black Friday el tráfico se triplica durante 30 segundos: 5400 puntuaciones por segundo, unas 108 en curso, y 3600 eventos por segundo, 150 por segundo en cada una de 24 particiones. Cuando el modelo se vuelve 80 ms más lento, una puntuación tarda unos 100 ms: eso solo ya supera el p95 de 60 ms, y 1800 × 0,1 s mantienen unos 180 pagos esperando a la vez. La pregunta es cuánto está dispuesta a esperar la API.

## El diseño de referencia

Los Servidores de socio (B2B) envían cada solicitud a un AWS ALB, que la reparte entre 6 instancias de API Go con un Tiempo límite de llamada de 40 ms. Para una puntuación, la API lee las características de la tarjeta de una caché Redis con 16 GB de Memoria y después llama a una tarjeta de Inferencia de modelos NVIDIA Triton con 2 instancias y el Tipo de modelo en Clasificador. Para un evento, la API publica en un stream Apache Kafka con 24 Particiones; Apache Flink, con 2 instancias, lo consume y escribe las características actualizadas en el mismo Redis. Cuesta unos 2401 $ al mes, y la mayor parte son las GPU: Triton cuesta 1468 $, la API 420 $, Redis 195 $, Kafka 153 $, Flink 140 $ y el ALB 25 $. En un día normal el p95 es de unos 28 ms, sin errores, y las GPU están ocupadas en torno al 13 %.

«El servidor del modelo se vuelve lento» añade 80 ms a cada llamada al modelo a partir del segundo 20, durante 30 segundos. Con el Tiempo límite de llamada, la API deja de esperar a los 40 ms: la puntuación tardía se descarta y deciden las reglas, así que el pago sigue teniendo éxito. El p95 sube a unos 53 ms, sin errores.

«Black Friday» triplica el tráfico a partir del segundo 15, durante 30 segundos. Hay margen en todas partes: el p95 se queda en unos 28 ms, sin errores.

Este problema no tiene reto de coste. Las dos GPU son más de la mitad de la factura, y el diseño más barato encontrado que supera todas las pruebas sigue costando en torno al 84 % de la referencia, por encima del tope del reto.

## Por qué falla el primer intento

Un primer intento habitual es exactamente el mismo diseño sin Tiempo límite de llamada. Cuesta los mismos 2401 $, supera «Un día normal» con un p95 de unos 28 ms y también supera «Black Friday».

Cuando el modelo se vuelve lento, falla: un 8,7 % de errores y un p95 de unos 1,6 s. Sin límite, cada pago espera los 80 ms de más por su puntuación. Cada pago que espera retiene un hueco de la API, los huecos se llenan, las solicitudes nuevas se encolan detrás y muchas fallan. El paso del modelo siempre fue opcional, pero nada le decía a la API que dejara de esperarlo.

La idea es esta: una llamada a un modelo de ML en línea necesita un presupuesto de latencia. Un tiempo límite con reglas de respaldo mantiene rápido el pago cuando el modelo va lento. Por lo demás, los dos diseños son idénticos; un solo ajuste marca la diferencia.

## Lo que el simulador deja fuera

No se simula la precisión del modelo, ni la calidad de las reglas de respaldo: un pago decidido por reglas cuenta igual que uno que puntuó el modelo. Tampoco se mide la frescura de las características, es decir, con cuánto retraso llegan a Redis las actualizaciones de Flink. Los importes de los pagos y las devoluciones de cargo tampoco se simulan.

## Para probar

- Quita el Tiempo límite de llamada y vuelve a ejecutar «El servidor del modelo se vuelve lento». Mira cómo se llenan los huecos de la API mientras el modelo va lento.
- Pon el Tiempo límite de llamada en 15 ms, por debajo de los 20 ms habituales del modelo, y ejecuta «Un día normal». ¿Cuántos pagos deciden ahora las reglas?
- Baja Triton a 1 instancia y ejecuta «Black Friday». Mira lo ocupada que llega a estar la GPU y si se mueve el p95.
