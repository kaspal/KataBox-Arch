# Solución comentada

## Adónde va la carga

Un turno son tres llamadas a modelos seguidas, y el Servidor de aplicaciones espera a cada una antes de empezar la siguiente. El usuario oye la suma. Con los valores por defecto de las tarjetas y un turno típico (un prompt de 300 tokens y una respuesta de 15), las etapas tardan:

- la voz a texto en Triton: 5 ms + 300 ÷ 5000 tokens/s = 65 ms;
- el modelo de lenguaje en vLLM: 200 ms para arrancar, 300 ÷ 5000 = 60 ms para leer el prompt y 15 ÷ 80 tokens/s ≈ 188 ms para escribir la respuesta, unos 450 ms en total;
- el texto a voz en Triton: 5 ms + 300 ÷ 50 000 = 6 ms, más una unidad de salida, unos 24 ms.

Son unos 540 ms. Un turno largo (450 tokens de entrada, 30 de salida) tarda unos 95 + 665 + 26 ≈ 790 ms. Casi todo es el modelo de lenguaje, y casi todo eso es escribir la respuesta.

Dos cosas se comen el resto de los 1,2 s. La primera es el tiempo de arranque: el Tiempo hasta el primer token de las API alojadas es de 500 a 600 ms por defecto, frente a los 200 ms de vLLM, así que un modelo de lenguaje alojado gasta más o menos la mitad del presupuesto antes de escribir una palabra. La segunda es la propia GPU. Una GPU con vLLM tiene 16 huecos, y cada petición escribe a 80 × (1 − 0,5 × ocupados ÷ huecos) tokens por segundo. A 20 turnos por segundo, con cada llamada al modelo de lenguaje tardando unos 0,5 a 0,6 s, hay unos 12 huecos ocupados en cada momento. En una sola GPU eso son tres cuartos de sus huecos, así que cada respuesta se escribe a unos 50 tokens por segundo en lugar de 80, y la respuesta larga tarda 600 ms en lugar de 375.

Las pruebas ocultas se llevan el tiempo de sobra. «Un modelo va 300 ms más lento» suma 300 ms a cada llamada de una tarjeta de Inferencia de modelos (la primera que se colocó en el diseño; en los diseños de aquí, el modelo de lenguaje) desde el segundo 15, durante 40 segundos. «El triple de turnos durante 30 s» envía 60 turnos por segundo del segundo 15 al 45: unos 30 huecos ocupados.

## El diseño de referencia

Un Cliente móvil mantiene su sesión en un Gateway de WebSocket Socket.IO con 2 instancias, que con sus valores por defecto aguanta 200 000 conexiones (100 000 cada una), más que las 150 000 sesiones. El gateway pasa los turnos a 2 Servidores de aplicaciones Go. Estos llaman a un NVIDIA Triton con el Tipo de modelo «Embeddings» para la voz a texto, a un vLLM en GPU con 4 instancias para el modelo de lenguaje, y a un NVIDIA Triton con el Tipo de modelo «Clasificador» y una Velocidad del prompt de 50 000 tokens/s para el texto a voz. Cuesta unos 6152 $ al mes: 2936 $ por las 4 GPU del modelo de lenguaje, 1468 $ por cada tarjeta Triton (2 GPU cada una), 140 $ por los Servidores de aplicaciones y 140 $ por el gateway.

En una tarde normal, los 64 huecos del modelo de lenguaje están ocupados en torno al 15 %, así que las respuestas se escriben casi a los 80 tokens por segundo completos. El p95 es de unos 818 ms, sin errores y con todas las sesiones conectadas.

Cuando el modelo de lenguaje va 300 ms más lento, cada llamada ocupa su hueco 0,3 s más, unos 6 huecos ocupados más: todavía menos de un tercio de 64. El p95 sube a unos 1079 ms, por debajo del límite, sin errores. Si la lentitud cayera en una de las tarjetas de voz, sumaría los mismos 300 ms y nada más.

Cuando los turnos se triplican, hay unos 30 de los 64 huecos ocupados, y las respuestas se escriben a unos tres cuartos de la velocidad completa. El p95 es de unos 851 ms, sin errores.

Para el reto de coste, un diseño de prueba se queda en 4614 $, el 75 % del coste de la referencia, justo en el objetivo del 75 %. Mantiene la misma forma con 3 GPU para el modelo de lenguaje en lugar de 4, una instancia de Triton para la voz a texto y una instancia de Servidor de aplicaciones.

## Por qué falla el primer intento

Un primer intento habitual es el mismo diseño con una sola GPU para el modelo de lenguaje: la carga media cabe en sus 16 huecos, y solo está ocupada en torno a tres cuartos. Cuesta unos 3950 $ y pasa «Una tarde normal» con un p95 de unos 1145 ms. La pasa por poco: en 15 de los 60 segundos, el p95 de ese segundo supera 1,2 s, porque cada respuesta se escribe a unos 50 tokens por segundo.

Cuando el modelo de lenguaje va 300 ms más lento, hay 6 huecos ocupados más, y 12 + 6 es más que 16. Los turnos empiezan a hacer cola para conseguir un hueco. El p95 sube segundo a segundo, de unos 1,9 s en el segundo 19 a unos 4,8 s en el segundo 38, y desde el segundo 41 la app abandona turnos a los 5 s. Fallan en torno al 29,8 % de los turnos.

Cuando los turnos se triplican, hacen falta unos 30 huecos y hay 16. La cola crece desde el primer segundo, y desde el segundo 22 la mayoría de los turnos agotan el tiempo. Fallan en torno al 73,5 % de los turnos.

Llevar el modelo de lenguaje a una API alojada quita los huecos, pero no el problema. Con una tarjeta API de Gemini (500 ms hasta el primer token, 120 tokens por segundo) y sus límites subidos, una tarde normal tiene un p95 de unos 939 ms, y la lentitud de 300 ms lo lleva a unos 1215 ms, por encima del límite. Además cuesta unos 31 500 $ al mes, porque se cobra cada token: una tarjeta API de Claude es más lenta (p95 de unos 1168 ms) y cuesta unos 63 300 $. Dos GPU pasan una tarde normal (unos 868 ms) y la lentitud (unos 1168 ms, por poco), y aun así fallan el triple de turnos con en torno al 31,6 % de errores.

La idea es esta: un turno de voz es una cadena, y su presupuesto es la suma de todas las etapas, incluido el tiempo que cada una espera para arrancar y el que pasa en una GPU ocupada. Gasta lo menos posible en cada etapa (modelos que arrancan rápido, respuestas cortas, el primer trozo de audio en lugar del audio entero) y mantén las GPU casi libres, para que una etapa lenta o una multitud tengan tiempo de sobra que usar.

## Lo que el simulador deja fuera

El streaming entre etapas no se simula. Aquí cada etapa espera a que termine la anterior: el modelo de lenguaje escribe la respuesta entera antes de que empiece el texto a voz. Un sistema de voz real hace streaming: la voz a texto transcribe mientras el usuario todavía habla, las primeras palabras del modelo de lenguaje pasan al texto a voz en cuanto se escriben, y la respuesta suena mientras se escribe el resto. Con una respuesta de 30 tokens y el texto a voz empezando tras los 6 primeros, se ahorran unos (30 − 6) ÷ 80 ≈ 300 ms. Este problema lo sustituye por lo que el simulador sí modela: respuestas cortas, el texto a voz contado hasta su primer trozo de audio, modelos que arrancan rápido y margen en las GPU. Sus tiempos de turno son los de una cadena que se pasa resultados terminados, así que una cadena con streaming sería más rápida de lo que se ve aquí.

La calidad de las transcripciones, las respuestas y las voces no se simula, y nada de esto dice qué modelo suena o responde mejor. Los modelos de voz son tarjetas de Inferencia de modelos con los tipos «Embeddings» y «Clasificador», que solo representan sus tiempos: el simulador saca un tamaño de prompt por turno y se lo cobra a las tres llamadas. El audio en sí, la red desde el teléfono, la detección del final del habla, los usuarios que interrumpen la respuesta y la memoria de las GPU no se simulan. El tiempo hasta el primer token se registra en cada llamada a un modelo, así que este problema juzga la latencia del turno. El precio de la GPU es una cifra de tipo g5.xlarge, y los precios por token son las cifras didácticas del registro.

## Cosas que probar

- Pon el modelo de lenguaje en 1 GPU y ejecuta «Una tarde normal». Mira los segundos cuyo p95 supera 1,2 s, y calcula la velocidad de respuesta con 12 de 16 huecos ocupados.
- Cambia la tarjeta vLLM por una tarjeta API de Gemini y sube sus límites. Compara el p95 y el coste mensual con la referencia.
- Pon el modelo de lenguaje en 2 GPU y ejecuta «El triple de turnos durante 30 s». Cuenta los huecos ocupados que necesita.
