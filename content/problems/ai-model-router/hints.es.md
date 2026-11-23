## 1

Cuenta los tokens por minuto que llegan a la clave del modelo grande en un día normal, y otra vez cuando se triplican las preguntas fáciles. ¿Qué preguntas necesitan de verdad el modelo grande?

## 2

Las preguntas fáciles son el 80 % del tráfico y más de la mitad de los tokens del modelo grande. Envíalas al modelo pequeño en tus propias GPU: dejan de contar contra los 1 500 000 tokens por minuto de la clave, y una GPU cuesta lo mismo sirva 8 preguntas por segundo o 24. Después dimensiona esas GPU para el boletín: una respuesta fácil ocupa un hueco de lote durante un segundo y medio más o menos, y una GPU cuyos huecos se llenan hace más lenta cada respuesta.

## 3

Pon un API Gateway Kong delante de 2 Servidores de aplicaciones Go y 2 Microservicios. Conecta los Servidores de aplicaciones a una tarjeta vLLM en GPU con 4 Instancias y la Velocidad de salida en 150, y los Microservicios a una tarjeta API de Claude con el Límite de tokens por minuto en 1 500 000. En el Cliente web, pon el Tiempo límite en 60 000 ms.
