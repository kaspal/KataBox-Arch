# Solución comentada

## Adónde va la carga

Con 500 correos por segundo, el trabajo es esperar al proveedor: un sender ocupa un hueco en cada llamada. Solo con la mediana, 500 × 0,15 s mantienen ocupados unos 75 huecos, y la cola lenta, de hasta 1,2 s, añade muchos más. El límite del proveedor, 800 llamadas por segundo, deja margen en un día normal.

Las pruebas ocultas se llevan ese margen. Cuando el límite baja a 250 llamadas por segundo, siguen llegando 500 correos cada segundo, así que se rechazan unos 250 por segundo. En 40 segundos son unos 10 000 correos que todavía no pueden pasar. Cuando se solapan dos campañas, llegan 1000 correos por segundo, más que los 800 del proveedor. La pregunta es si esos correos seguirán ahí cuando puedan salir.

## El diseño de referencia

Un Campaign scheduler, una tarjeta Airflow que se dispara cada segundo, publica los correos en una Queue SQS con max_retries en 10. Desde esa cola sale una Dead-letter Queue. La consumen 4 instancias Celery de Mail senders, cada una con concurrencia 64: 256 huecos en total, ocupados en unos dos tercios en un día normal. Cuesta unos 865 $ al mes, la mayor parte por la cola, y el p95 de la campaña normal es de unos 786 ms.

«El proveedor te baja el límite» reduce el límite a 250 llamadas por segundo a partir del segundo 20, durante 40 segundos, en una ejecución de 120 segundos. Admite un p95 de 120 segundos y ningún correo perdido. Las llamadas rechazadas que se reintentan no son errores: el correo sigue en camino, así que Resultados las cuenta aparte como intentos reintentados. Cada correo rechazado vuelve a la cola y se reintenta tras 1, 2, 4, 8, 16, 32, 32, 32, 32 y 32 segundos: 191 segundos en total, mucho más que el recorte. El atasco se vacía cuando el límite vuelve a 800: el p95 queda en unos 25 s, sin perder nada. Lo que aún fallara acabaría en la Dead-letter Queue.

«Se solapan dos campañas» duplica el tráfico durante 30 segundos, a partir del segundo 20, con los mismos límites. Los correos de más esperan en la cola y salen a medida que lo permiten los senders y el proveedor, y el p95 queda en unos 9,7 s.

## Por qué falla el primer intento

Un primer intento habitual es el mismo diseño con los 3 reintentos por defecto de la cola y sin Dead-letter Queue. Cuesta lo mismo y supera «Una campaña normal» con el mismo p95.

Durante el recorte, un correo rechazado se reintenta tras 1, 2 y 4 segundos. Esas esperas suman solo 7 segundos, mientras que el recorte dura 40, así que un correo que se encuentra una y otra vez con el límite lleno agota sus reintentos y se descarta: se pierden unos 1891 correos. La prueba del solapamiento sigue superándose, con el mismo p95 de unos 9,7 s y nada perdido: ahí el proveedor sigue aceptando 800 llamadas por segundo, así que los correos rechazados encuentran hueco dentro de sus tres reintentos.

La idea es esta: los reintentos reparten la carga en el tiempo, así que un límite más bajo significa una campaña más lenta, no correo perdido, siempre que los reintentos duren más que el problema. Una Dead-letter Queue guarda lo que aún falle.

## Lo que el simulador deja fuera

Los proveedores reales envían una cabecera Retry-After con el 429 y limitan el ritmo por dominio de destino; aquí la única forma de marcar el ritmo son las esperas de la cola. Tampoco se simulan los rebotes, las bajas ni la reputación del remitente.

## Para probar

- Pon 5 reintentos máximos (31 segundos de esperas) y vuelve a ejecutar el recorte. ¿Bastan para un recorte de 40 segundos?
- Mantén 10 reintentos pero quita la Dead-letter Queue, y compara los resultados de los dos diseños.
- Sube los Mail senders a 8 instancias y vuelve a ejecutar el solapamiento. Mira si baja el p95 o si ahora lo que manda es el límite del proveedor.
