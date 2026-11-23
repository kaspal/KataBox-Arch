# Solución comentada

## Adónde va la carga

Dos cosas deciden este diseño: dónde se escribe el historial, y cuántas GPU quedan cuando se pierde algo.

Primero, el historial. Cada mensaje hace dos escrituras, el mensaje del usuario y la respuesta. En el pico son 30 × 2 = 60 escrituras por segundo, y 24 de ellas son de usuarios de la UE. La tarjeta DNS reparte a los usuarios entre las regiones sin mirar dónde viven, así que un mensaje de la UE se sirve en us-east más o menos la mitad de las veces. La única forma de mantener el historial de la UE en la UE es que todos los Servidores de aplicaciones, se ejecuten donde se ejecuten, escriban en una base de datos de una Región `eu`. El historial de los usuarios de EE. UU. no tiene regla, así que una sola base de datos de la UE puede guardar el de todos. Una escritura de us-east a la UE cuesta unos 70 ms en cada sentido, poco al lado de una respuesta que tarda segundos.

Una base de datos tiene un primario y réplicas, y la regla sigue al trabajo del primario. Cuando se pierde el primario, se promueve una réplica, y desde ese momento todas las escrituras caen en ella. Así que la Región de la réplica importa tanto como la del primario: también debe llevar la etiqueta `eu`, y debe ser una Región distinta de la del primario, o perder una región se lleva las dos.

Después, las GPU. Antes del primer token, un mensaje necesita 200 ms más su prompt a 5000 tokens por segundo: 0,2 + 800 / 5000 = 0,36 s para un prompt típico, 0,6 s para uno largo, más unos 70 ms en cada sentido cuando el Servidor de aplicaciones llama a una GPU de la otra Región. El límite es 1 s. Después la respuesta se escribe a 80 tokens por segundo sola y a 40 con un lote lleno de 64, así que una GPU escribe como mucho 64 × 40 = 2560 tokens por segundo. El pico pide 30 × 250 = 7500 tokens por segundo, y algo más en la práctica, porque las respuestas largas suben la media por encima de los 250 típicos. Tres GPU al 100 % no llegan del todo; cuatro van llenas más o menos en tres cuartos.

En un día normal, dos grupos se reparten ese trabajo. En un mal día, un grupo lo hace solo: cuando se apaga eu-west, o cuando se detiene una tarjeta de vLLM. Así que cada grupo debe llevar el pico entero por sí solo.

## El diseño de referencia

Un Cliente web con el Tiempo límite en 30 000 ms, 2 Reintentos, la Espera entre reintentos en 200 ms y la Variación de la espera en Completa (espera aleatoria) llama a una tarjeta DNS Route 53 (TTL 30 s, Round robin). Esta apunta a dos regiones:

- **us-east** (Etiqueta de residencia `us`): un AWS ALB, 4 Servidores de aplicaciones Go y una tarjeta vLLM en GPU con 4 Instancias y los Huecos de lote por GPU en 64.
- **eu-west** (Etiqueta de residencia `eu`): lo mismo, más el primario PostgreSQL del historial.

Una tercera Región, **eu-central** (Etiqueta de residencia `eu`), tiene una réplica PostgreSQL, conectada desde el primario con una conexión Replicar. Las dos bases de datos tienen el Tiempo de failover en 10 s. Los dos grupos de Servidores de aplicaciones escriben en el primario con conexiones Consulta, y cada grupo está conectado a las dos tarjetas de vLLM.

Cuesta unos 7182 $ al mes: 5872 $ por las 8 GPU, 700 $ por las dos bases de datos, 560 $ por los 8 Servidores de aplicaciones y 50 $ por los dos ALB.

En un día normal, cada grupo de GPU está ocupado más o menos al 24 %, el p95 del tiempo hasta el primer token (TTFT) es de unos 0,77 s, el p95 de la respuesta entera de unos 9,7 s, y no hay errores ni infracciones de residencia.

"Se apaga la región de la base de datos del historial" pierde eu-west a los 30 segundos de una ejecución de 5 minutos, para siempre: su ALB, sus Servidores de aplicaciones, sus GPU y el primario. Durante unos segundos la tarjeta DNS sigue enviando allí a la mitad de los usuarios, hasta que dos comprobaciones de salud seguidas la encuentran caída, y cada escritura espera a que la réplica tome el relevo 10 s después. Esos segundos dan alrededor de un 4,6 % de errores, la mayoría reintentados por el cliente. Después la réplica de eu-central es el primario, todavía en la UE, y las GPU de us-east llevan los 30 mensajes por segundo: el p95 del TTFT es de unos 0,76 s, el de la respuesta entera de unos 12,1 s, y no hay infracciones de residencia.

"Un día de auditoría solo con la UE: falla el primario de la base de datos y después un grupo de GPU" envía todos los mensajes desde usuarios de la UE al mismo ritmo total, así que cada una de las 60 escrituras por segundo debe caer en la UE. El primario se detiene a los 30 segundos, para siempre, y la réplica de eu-central toma el relevo; después, desde los 100 segundos, la primera tarjeta de vLLM (por id de tarjeta) se detiene durante 60 segundos, y los Servidores de aplicaciones envían todas las llamadas al modelo a la otra tarjeta. Alrededor de un 4,7 % de los mensajes fallan en torno a los dos fallos, el p95 del TTFT es de unos 0,82 s, el de la respuesta entera de unos 10,5 s, y no hay infracciones de residencia.

Este problema no tiene reto de coste: una búsqueda de un diseño más barato que supere todas las pruebas no encontró ninguno por debajo del 80 % del coste de la referencia. Los diseños más baratos que sí pasan dejan un grupo de GPU más pequeño justo donde la prueba de GPU golpea por casualidad, y eso funciona por suerte con los nombres de las tarjetas, no por diseño.

## Por qué falla el primer intento

Un primer intento habitual acierta casi en todo. Tiene las mismas dos regiones, las mismas GPU y el mismo primario en eu-west, y pone la réplica de la base de datos en us-east, junto a los servidores de EE. UU. de la empresa, como copia para recuperación ante desastres. Cuesta los mismos 7182 $ al mes, y supera "Un día normal" con los números de la referencia: todas las escrituras van al primario de eu-west, así que ninguna cae fuera de la UE.

Cuando se apaga eu-west, se promueve la réplica de us-east, y desde ese momento las dos escrituras de cada mensaje de la UE caen en una Región con la etiqueta `us`. Los errores son los de la referencia (alrededor de un 4,6 %), la latencia está bien, y la prueba falla de todos modos: unas 6200 infracciones de residencia, todos los mensajes de la UE de los últimos cuatro minutos de la ejecución. En la prueba de auditoría, cuando se detiene el primario, ocurre la misma promoción con solo tráfico de la UE: unas 15 600 infracciones. La recuperación ante desastres llevó los datos justo al sitio al que nunca deben ir.

Otros primeros intentos fallan de formas instructivas:

- **Todo en us-east.** Una sola Región de EE. UU. con las GPU y la base de datos falla "Un día normal" desde el principio: cada escritura de la UE es una infracción.
- **Sin réplica.** Un solo PostgreSQL en eu-west (6832 $) supera el día normal y pierde todas las escrituras cuando cae: alrededor de un 91 % de errores en las dos pruebas ocultas.
- **Grupos dimensionados para sus propios usuarios.** Dos tarjetas de vLLM de 3 GPU cada una (5714 $) superan el día normal. Cuando se apaga eu-west, 3 GPU reciben el pico entero, más tokens por segundo de los que pueden escribir: los lotes se llenan, los mensajes hacen cola, y la prueba termina con alrededor de un 41,6 % de errores y un p95 de unos 28 s. La prueba de auditoría falla por latencia (un p95 de unos 17,6 s) mientras una tarjeta está detenida.
- **Una sola tarjeta de GPU grande.** Las 8 GPU en una sola tarjeta de vLLM en us-east cuestan lo mismo y superan la prueba de la región, porque us-east sigue funcionando. Cuando se detiene la tarjeta, no queda nada: alrededor de un 26 % de errores en la prueba de auditoría.
- **Cada grupo de Servidores de aplicaciones conectado solo a las GPU de su Región.** Pasa por poco: mientras la tarjeta de eu-west está detenida, la tarjeta DNS sigue enviando a la mitad de los usuarios a Servidores de aplicaciones sin GPU a la que llamar, y solo los reintentos del cliente que caen en us-east reciben respuesta (alrededor de un 7,2 % de errores, con un límite del 8 %).

La idea es esta: una regla de residencia trata de todos los sitios donde pueden acabar los datos, y el failover es uno de esos sitios. Pon la réplica, y cualquier otra cosa que pueda hacer el trabajo del primario, en una Región con la misma etiqueta. Y una Región que puedes perder es capacidad que puedes perder: cada grupo de GPU debe llevar el pico entero por sí solo.

## Lo que el simulador deja fuera

La calidad de las respuestas no se simula: una GPU de cualquiera de las dos regiones da la misma respuesta. Los usuarios no tienen ubicación: una tarjeta DNS los reparte por turnos entre las regiones, sin enrutar por geografía, así que los usuarios de la UE no reciben respuestas más rápidas desde eu-west, y el simulador no puede enviar sus mensajes allí a propósito. La residencia solo se comprueba cuando una base de datos, una caché o una cola guarda o sirve una petición; copiar datos por replicación no cuenta, así que la réplica del primer intento no rompe ninguna regla en el simulador hasta que se promueve, mientras que en la realidad guarda copias del historial de la UE desde la primera fila que copia. Las llamadas al modelo, los logs, las copias de seguridad y las métricas no se comprueban en absoluto. Las reglas de residencia reales vienen de contratos y de leyes como el RGPD, que a menudo cubren también el procesamiento; la regla de este problema es la de su enunciado, no un consejo legal.

La memoria de las GPU no se simula: el límite de 64 respuestas a la vez en una GPU viene del enunciado. El failover de 10 s lo fija el enunciado; el tiempo de failover de una base de datos real depende de cómo esté configurada. Una Región se pierde entera de golpe y sigue perdida; las caídas reales suelen ser parciales, y pueden durar horas donde la prueba dura 5 minutos. Arrancar servidores de GPU y cargar el modelo no se simula, y una tarjeta de Inferencia de modelos no se puede autoescalar aquí. El precio de las GPU es la cifra didáctica del registro (un nodo de GPU a 734 $ al mes), no un presupuesto.

## Para probar

- Mueve la réplica del primer intento de us-east a una Región nueva con la etiqueta `eu` y vuelve a ejecutar las dos pruebas ocultas.
- Dale 3 Instancias a cada tarjeta de vLLM de la referencia y ejecuta "Se apaga la región de la base de datos del historial". Mira cómo se llenan los lotes.
- Quita las conexiones de cada grupo de Servidores de aplicaciones a la tarjeta de vLLM de la otra Región y compara los errores de la prueba de auditoría con los de la referencia.
