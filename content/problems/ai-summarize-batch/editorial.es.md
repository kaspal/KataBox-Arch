# Solución comentada

## Adónde va la carga

Cada documento tiene unos 600 tokens de entrada y 100 de salida, así que unos 700 tokens por llamada. El atraso de esta noche son 450 documentos: unos 315 000 tokens. Tu cuenta permite 400 000 tokens por minuto en tiempo real, y el límite empieza con la asignación de un minuto completo, así que el atraso de esta noche cabe con margen. El límite de peticiones (4000 por minuto) nunca es el problema aquí; con 700 tokens por llamada, el límite de tokens se agota antes.

El triple de atraso son 1350 documentos, unos 945 000 tokens. Los primeros 400 000 pasan de golpe; el resto entra a 400 000 por minuto, unos 9,5 documentos por segundo, así que los últimos documentos esperan unos 80 segundos más. El plazo es de 45 segundos. Ningún número de Workers cambia eso: el límite está del lado del proveedor.

El otro tope son tus propios Workers. Una llamada tarda unos 2,2 segundos (1100 ms hasta el primer token, 120 ms para leer el prompt, más o menos 1 s para escribir 100 tokens a 100 tokens por segundo). 2 instancias de Celery con una concurrencia de 64 sostienen 128 llamadas a la vez, unos 58 documentos por segundo, más que los 45 por segundo de la prueba más grande.

## El diseño de referencia

Un Planificador Airflow publica cada documento en una Cola Amazon SQS (Reintentos máximos 10), conectada a una Cola de mensajes fallidos. 2 Workers Celery con una concurrencia de 64 la consumen y llaman a una tarjeta de Inferencia de modelos API de Claude con el "Modo de API" en "Lotes (más barato, más lento)". En modo lotes el precio por token es la mitad y los límites por minuto no se aplican. Cuesta unos 12 564 $ al mes: unos 12 391 $ de tokens, 140 $ de los Workers, 20 $ del Planificador y 13 $ de la Cola. El atraso de esta noche tiene un p95 de unos 3,2 s desde la cola hasta el resumen, sin errores y sin nada perdido.

"El triple de atraso" encola 45 documentos por segundo durante 30 segundos. Sin límite por minuto, los Workers los toman tan rápido como llegan (unas 100 llamadas a la vez, de 128), y el p95 se queda en unos 3,5 s.

"El proveedor del modelo se vuelve lento" añade 3 segundos a cada llamada al modelo durante los primeros 90 segundos, con el atraso normal. Una llamada tarda ahora unos 5,2 segundos, así que 15 documentos por segundo necesitan unas 78 llamadas a la vez, todavía por debajo de 128. El p95 es de unos 6,4 s, sin errores.

Para el reto de coste no hay objetivo: la búsqueda no encontró nada útilmente más barato que la referencia (su mejor prueba costaba el 100 % de ella). Casi toda la factura son tokens, y el modo lotes ya los ha reducido a la mitad.

## Por qué falla el primer intento

Un primer intento habitual es el mismo diseño con el modelo en tiempo real, el valor por defecto de la tarjeta. Cuesta unos 24 955 $ al mes, el doble en tokens, y supera "El atraso de esta noche" con el mismo p95 de unos 3,2 s, porque 315 000 tokens caben en la asignación del primer minuto. También supera "El proveedor del modelo se vuelve lento" (p95 de unos 6,4 s): un proveedor más lento no gasta más tokens.

"El triple de atraso" lo rompe. En cuanto se agotan los tokens del primer minuto, las llamadas vuelven con un HTTP 429. Una llamada rechazada hace fallar el mensaje, y la Cola lo reintenta tras una espera, mientras siguen llegando documentos nuevos. Los documentos esperan detrás del límite de tokens, y el p95 sube por encima de un minuto, frente a un plazo de 45 segundos. Los Workers están ocupados un 7 % en una noche normal: no son el cuello de botella. Con el mismo diseño y el límite de tokens levantado, la prueba se supera, lo que demuestra que la causa es el límite. Menos reintentos tampoco lo salvan: con los "Reintentos máximos" de la Cola en 3, unos 371 documentos se rinden y van a la Cola de mensajes fallidos, y la prueba falla por ellos, porque ninguno se resumió.

La idea es esta: cuando nadie espera una respuesta, la API en tiempo real te vende una velocidad que no necesitas, a precio completo, y encima con un límite por minuto. El trabajo con plazo va por la API por lotes: mitad del precio por token y sin límite por minuto, y entonces el único tope es la capacidad de Workers que dimensionas tú.

## Lo que el simulador deja fuera

Una API por lotes real no responde en segundos: envías un lote, y los resultados vuelven más tarde, dentro de una ventana que fija el proveedor. En este simulador una llamada por lotes tarda lo mismo que una en tiempo real: su ventana de entrega no se simula como tiempo (el simulador solo rechaza las llamadas por lotes de un trabajo que alguien está esperando, y este atraso está marcado como trabajo que puede esperar), y por eso funciona el plazo escalado de 45 segundos. En un trabajo real, comprueba que la ventana de lotes del proveedor cabe en tu plazo. Las API por lotes también tienen límites propios (cuántas peticiones puede tener un lote o una cola), que no se simulan. El simulador no te impide subir el "Límite de tokens por minuto" de la tarjeta; un proveedor real sí. La calidad de los resúmenes no se mide en absoluto. Los descuentos por caché de prompts, las cabeceras Retry-After y el coste de guardar los resúmenes no se simulan, y el coste mensual es el ritmo de la ejecución mantenido un mes, no la factura de un trabajo nocturno.

## Para probar

- Cambia el modelo a "Tiempo real" y ejecuta "El triple de atraso". Mira cuándo empiezan los 429 y compáralo con 400 000 tokens por minuto.
- En tiempo real, duplica la concurrencia de los Workers y vuelve a ejecutarla. Más huecos solo mandan más llamadas contra el mismo límite.
- En modo lotes, baja los Workers a 1 instancia con una concurrencia de 32 y ejecuta "El proveedor del modelo se vuelve lento". Compara las llamadas en curso con documentos por segundo × segundos por llamada.
