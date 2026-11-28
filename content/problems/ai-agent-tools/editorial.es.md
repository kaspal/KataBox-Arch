# Solución comentada

## Adónde va la carga

Una tarea de agente es una cadena de llamadas, y el Worker espera cada una mientras conserva su hueco. Una llamada al modelo con los tamaños medianos tarda unos 2,4 s: 1100 ms hasta el primer token, unos 0,3 s para leer 1500 tokens a 5000 por segundo y alrededor de 1 s para escribir 100 tokens a 100 por segundo. Una larga (3000 tokens de entrada, 250 de salida) tarda unos 4,2 s. Las herramientas son rápidas en comparación: de decenas a cientos de milisegundos.

Así que una pregunta (dos llamadas al modelo) tarda unos 5,2 s, y un reembolso (cuatro) unos 10,2 s. Con 2 tareas por segundo, un 80 % de ellas preguntas, una tarjeta Worker tiene ocupados a la vez unos 2 × (0,8 × 5,2 + 0,2 × 10,2) ≈ 12 huecos. Un Worker Celery tiene 8 huecos por instancia.

Los límites del modelo no son el problema aquí. 2,4 llamadas al modelo por tarea son unas 290 llamadas por minuto frente a 4000, y unos 460 000 tokens por minuto con los tamaños medianos (las llamadas largas suben la media real alrededor de una cuarta parte) frente a 2 000 000.

Lo que rompe un diseño es el tiempo. Cuando el seguimiento tarda 40 s por llamada, una tarea que lo espera retiene su hueco 40 s más: unos 2 × 46 ≈ 92 huecos ocupados a la vez. Y en la semana de devoluciones cambia la mezcla: 1,6 preguntas y 2 reembolsos por segundo necesitan unos 1,6 × 5,2 + 2 × 10,2 ≈ 29 huecos.

## El diseño de referencia

Un Cliente web envía las tareas a un Servidor de aplicaciones Go, que las publica en una Cola Amazon SQS con los Reintentos máximos en 6. Un Worker Celery con 5 Instancias (40 huecos) y el Tiempo límite de llamada en 8000 ms las consume y llama a una tarjeta de Inferencia de modelos API de Claude con el Límite de tokens por minuto en 2 000 000. Cuesta unos 63 282 $ al mes, y los tokens son casi todo: unos 62 784 $ en la API de Claude, 350 $ del Worker, 140 $ del Servidor de aplicaciones y 7 $ de la Cola. En un día normal, el p95 hasta completar es de unos 12,1 s, sin errores, y el Worker está ocupado un 38 % más o menos.

El Tiempo límite de llamada es un solo número para todas las llamadas del Worker, así que tiene que dar cabida a la llamada más lenta sin la que la tarea no puede seguir: una llamada larga al modelo, unos 4,2 s. 8 s dejan margen para eso y aun así ponen un tope a una herramienta lenta. De vez en cuando una llamada larga al modelo sí lo supera; los 6 reintentos de la Cola le dan otra oportunidad a esa tarea en vez de perderla.

«La API de seguimiento del transportista tarda 40 s por llamada» añade 40 s a cada llamada de seguimiento a partir del segundo 20, durante 60 segundos, en una ejecución de 2 minutos. El Worker deja de esperar el seguimiento a los 8 s y, como el seguimiento es opcional, el agente sigue y responde con lo que dice el pedido. La llamada de seguimiento que abandonó sigue ejecutándose sin que nadie la quiera, pero ya no retiene ningún hueco. Una tarea tarda ahora unos 8 s más, así que durante la lentitud están ocupados la mayoría de los 40 huecos. El p95 es de unos 17,6 s, sin errores, y el Worker está ocupado un 45 % más o menos durante la ejecución.

«Semana de devoluciones: cinco veces los reembolsos» multiplica los reembolsos por 5 durante los 2 minutos de la ejecución. Con los tamaños medianos están ocupados unos 29 huecos, y los 40 huecos tienen sitio también para las tareas largas: el Worker está ocupado un 74 % más o menos, el p95 es de unos 15,6 s y no hay errores. La factura de tokens con esa mezcla es más o menos el doble que la de un día normal, y los 2 000 000 de tokens por minuto de la cuenta siguen cubriéndola.

Este problema no tiene reto de coste. Los tokens son más del 99 % de la factura y los fija el tráfico, así que el diseño más barato encontrado que supera todas las pruebas cuesta lo mismo que la referencia.

## Por qué falla el primer intento

Un primer intento habitual es la misma cadena con un Worker Celery de 3 Instancias (24 huecos) y sin Tiempo límite de llamada. Cuesta unos 63 142 $ y supera «Un día normal» con el mismo p95 de unos 12,1 s; su Worker está ocupado un 63 % más o menos.

Cuando el seguimiento se vuelve lento, falla con un p95 de más de un minuto. Cada tarea espera los 40 s enteros del seguimiento, así que cada una retiene su hueco unos 46 s en vez de 6. Los 24 huecos se llenan en pocos segundos, las tareas nuevas esperan en la Cola detrás de ellas, y la espera crece mientras dura la lentitud. A partir del segundo 64 más o menos, las tareas que terminan cada segundo tienen un p95 de entre 36 y 66 s. El Worker está ocupado un 86 % más o menos. Nada falla: el seguimiento sigue respondiendo, solo que demasiado tarde para servir de algo, y por eso el agente debería dejar de preguntar.

La semana de devoluciones también falla, con un p95 de unos 38,1 s: llega trabajo para unos 29 huecos y solo hay 24, así que la Cola crece y el Worker está ocupado un 96 % más o menos.

Arreglar una sola mitad no basta:

- El primer intento con el Tiempo límite de llamada en 10 000 ms supera la lentitud (un p95 de unos 22,8 s) y sigue fallando la semana de devoluciones con unos 38,1 s: un tiempo límite acota una tarea, pero no añade huecos.
- 5 Instancias sin Tiempo límite de llamada superan la semana de devoluciones y fallan la lentitud con un p95 de unos 62,5 s. Con 8 Instancias sigue fallando, con unos 52,3 s: unos 92 huecos de espera son más de lo que tiene sentido pagar.

La idea es esta: cada llamada que hace un agente necesita un tiempo límite, y los pasos de los que la tarea puede prescindir deben poder fallar. Así una herramienta lenta le cuesta a cada tarea como mucho su tiempo límite, en vez de retener todas las tareas y todos los huecos. Y dimensiona los workers para las cadenas más largas que esperas, con margen, porque cada paso de un agente retiene el hueco.

## Lo que el simulador deja fuera

La calidad de las respuestas no se simula: una respuesta escrita sin el resultado del seguimiento cuenta igual que una que lo tiene. Un agente real decide sus pasos sobre la marcha, así que el número de llamadas al modelo y a las herramientas varía de una tarea a otra; aquí cada tipo de tarea tiene una cadena fija. El Tiempo límite de llamada es un solo valor para todas las llamadas de un Worker; el código real suele poner un tiempo límite por herramienta y un presupuesto de tiempo o de pasos para la tarea entera. No se simulan las llamadas a herramientas en paralelo, los reintentos de una llamada concreta a una herramienta ni un tope de iteraciones del agente. Las herramientas no tienen precio aquí, y las llamadas de seguimiento que el Worker abandonó no cuestan nada. La lentitud dura 60 s en una ejecución de 2 minutos; una real puede durar mucho más, y entonces la cola acumulada del primer intento no se vacía nunca. Los precios por token son las cifras didácticas del registro.

## Para probar

- Quita el Tiempo límite de llamada de la referencia y ejecuta la lentitud. Mira cómo se llenan los huecos del Worker.
- Pon el Worker de la referencia en 3 Instancias y ejecuta la semana de devoluciones. ¿Cuántos huecos necesita la nueva mezcla?
- Baja el Límite de tokens por minuto a 400 000 y ejecuta la semana de devoluciones. ¿Qué límite se acaba antes ahora?
