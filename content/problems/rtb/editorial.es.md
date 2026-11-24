# Solución comentada

## Adónde va la carga

Con 20 000 subastas por segundo, el trabajo es esperar a los pujadores: un Servidor de aplicaciones ocupa un hueco en cada llamada. Solo con la mediana, 20 000 × 0,03 s mantienen ocupados unos 600 huecos, y la cola lenta de los pujadores, de hasta 90 ms en el p99, añade más. La lectura del perfil en Redis es rápida y suma poco.

Las pruebas ocultas se llevan ese margen. Cuando un pujador grande se vuelve lento, cada llamada a los pujadores tarda 80 ms más, unos 110 ms de mediana. Si la subasta lo espera, 20 000 × 0,11 s son unos 2200 huecos ocupados a la vez, y cada subasta supera el objetivo de 100 ms. En el horario de máxima audiencia llegan 40 000 subastas por segundo, así que incluso con los 30 ms normales necesitas unos 1200 huecos solo para la mediana.

## El diseño de referencia

Un AWS ALB reparte las subastas entre 8 Servidores de aplicaciones Go, cada uno con una Concurrencia por instancia de 256 y un Tiempo límite de llamada de 70 ms. Leen los perfiles de una Caché Redis con 16 GB de memoria. Cuesta unos 780 $ al mes: 560 $ por los Servidores de aplicaciones, 195 $ por Redis y 25 $ por el ALB. En una hora normal el p95 es de unos 76 ms y el p99 de unos 82 ms, sin errores, y los Servidores de aplicaciones están ocupados en torno a un 39 %.

«Un pujador grande se vuelve lento» añade 80 ms a los pujadores a partir del segundo 20, durante 30 segundos. El p95 sube a unos 86 ms y el p99 a unos 93 ms, sin errores: las pujas que tardan más de 70 ms se descartan, la subasta se cierra a tiempo con las pujas que tiene y el hueco vuelve a quedar libre. Como el paso de los pujadores es opcional, una puja descartada no es un error.

«Horario de máxima audiencia» duplica el tráfico a partir del segundo 15, durante 30 segundos. 2048 huecos bastan: el p95 se queda en unos 76 ms, sin errores.

Para el reto de coste, un diseño de prueba se queda en el 72 % del coste de la referencia, por debajo del objetivo del 75 %. Mantiene la misma forma y el mismo Tiempo límite de llamada, con menos instancias de Servidor de aplicaciones y un Redis mucho más pequeño.

## Por qué falla el primer intento

Un primer intento habitual son 4 Servidores de aplicaciones Go con una Concurrencia por instancia de 256, sin Tiempo límite de llamada, y el mismo Redis. Cuesta unos 500 $ y supera «Una hora normal»: un p95 de unos 76 ms y un p99 de unos 102 ms, con los Servidores de aplicaciones ocupados en torno a un 80 %.

Cuando un pujador grande se vuelve lento, cada subasta lo espera. Cada una ocupa un hueco 80 ms más, hacen falta unos 2200 huecos y solo hay 1024, así que los huecos se agotan, las peticiones hacen cola y después fallan: alrededor de un 28 % de errores, con un p95 de unos 1,1 s. El horario de máxima audiencia también falla, con un 24 % de errores y un p95 de unos 393 ms: con un 80 % de ocupación en una hora normal, no queda margen para el doble de tráfico.

La idea es esta: en una subasta, o en cualquier reparto con un plazo, el participante más lento marca tu latencia a menos que pongas un plazo. Un tiempo límite de llamada convierte a un pujador lento en una puja perdida, no en una subasta lenta, y libera el hueco para la siguiente.

## Lo que el simulador deja fuera

Aquí los pujadores son una sola llamada combinada, no pujadores separados con sus propias pujas. No se simulan los ingresos de la subasta ni las tasas de victoria, así que una puja descartada simplemente desaparece. Tampoco se simulan el tiempo límite propio del exchange, la lógica de segundo precio ni el ritmo de gasto de los anunciantes.

## Para probar

- Quita el Tiempo límite de llamada de la referencia y vuelve a ejecutar «Un pujador grande se vuelve lento». Mira cuánto se ocupan los Servidores de aplicaciones.
- Pon el Tiempo límite de llamada en 30 ms y después en 95 ms, y compara el p95 y el p99 en una hora normal.
- Baja los Servidores de aplicaciones a 5 instancias y vuelve a ejecutar «Horario de máxima audiencia». ¿Qué ocupación tienen en el pico?
