## 1

Mientras una herramienta se toma su tiempo, ¿qué hace el Worker con su hueco? Y si nada deja nunca de esperar, ¿cuánto puede durar una tarea?

## 2

Una tarjeta Worker tiene ocupados a la vez unos (tareas por segundo) × (segundos por tarea) huecos. Si el seguimiento tarda 40 s por llamada, cada tarea retiene su hueco 40 s más, y los huecos se acaban en pocos segundos. Un Tiempo límite de llamada en el Worker limita cuánto espera cada llamada, y como el seguimiento es opcional, el agente sigue sin él. Las llamadas al modelo tienen que seguir cabiendo en ese tiempo. Después vuelve a contar los huecos para un día con muchos reembolsos: cada reembolso hace el doble de llamadas al modelo.

## 3

Un Cliente web envía las tareas a un Servidor de aplicaciones Go, que las publica en una Cola Amazon SQS. Un Worker Celery con 5 Instancias y el Tiempo límite de llamada en 10 000 ms las consume y llama a una tarjeta de Inferencia de modelos API de Claude con el Límite de tokens por minuto en 2 000 000.
