## 1

Cuenta los tokens, no las peticiones. ¿Cuántos tokens necesita todo el atraso, y cuántos permite tu cuenta en un minuto? Luego hazte la misma pregunta para un atraso tres veces mayor. Nadie espera a un resumen concreto: ¿este trabajo necesita de verdad la API en tiempo real?

## 2

La tarjeta de Inferencia de modelos tiene un "Modo de API". En tiempo real, cada llamada cuenta para los límites por minuto, y añadir Workers solo añade llamadas rechazadas. En modo lotes el precio por token es la mitad y los límites por minuto no se aplican, así que el único tope que queda es cuántas llamadas pueden sostener a la vez tus propios Workers: documentos por segundo × segundos por llamada.

## 3

Planificador (Airflow) → Publish → Cola Amazon SQS (Reintentos máximos 10) → Consume → Worker Celery, 2 instancias con una concurrencia de 64. Conecta la Cola a una Cola de mensajes fallidos con Publish. El Worker llama por HTTP a una tarjeta de Inferencia de modelos API de Claude, con el "Modo de API" en "Lotes (más barato, más lento)".
