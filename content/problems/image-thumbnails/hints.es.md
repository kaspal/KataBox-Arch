## 1

Redimensionar una foto lleva una quinta parte de un segundo. ¿Qué debería hacer la subida mientras tanto, y qué le pasa al trabajo si la máquina que lo hace desaparece a medias?

## 2

Pon los trabajos de miniatura en una cola y deja que los Workers los tomen a su ritmo. Calcula cuántos huecos de Worker ocupa el pico y asegúrate de que, si pierdes una tarjeta de Workers, aún te quedan bastantes para no quedarte atrás.

## 3

Publica cada trabajo en una Queue SQS y conecta a ella dos tarjetas de Workers Celery separadas, cada una capaz de llevar sola todo el pico: unas 12 instancias con la concurrencia por defecto de 8. Si una tarjeta cae, la otra sigue, y el visibility timeout de la cola le devuelve los trabajos que la tarjeta caída no terminó.
