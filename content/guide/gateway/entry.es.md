Un API gateway es el único punto de entrada para las API de muchos servicios. Cada llamada pasa por él, y por el camino hace las tareas que toda API necesita: comprobar quién llama, enviar la llamada al servicio correcto, aplicar cuotas y registrarla. Así cada servicio puede centrarse en su propio trabajo.

Esa comodidad tiene un coste. El gateway hace trabajo real en cada petición, así que añade unos milisegundos, y hay que dimensionarlo como cualquier otra capa. Si se queda corto, se convierte en el cuello de botella de todo lo que tiene detrás.

En el simulador un gateway es un salto con su propia capacidad, instancias por concurrencia, y un tiempo de proceso por petición. Las cuotas por cliente las modela la tarjeta de Limitador de tasa, así que conecta una cuando el problema trate de abusos.
