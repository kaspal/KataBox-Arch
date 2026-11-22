El breaker vive dentro de quien llama, alrededor de las llamadas a una dependencia. Mientras está cerrado, las llamadas pasan y cuenta los fallos y timeouts. Cuando superan un umbral, se abre, y a partir de entonces las llamadas fallan al momento, o devuelven una respuesta de reserva, sin tocar la dependencia.

Tras un periodo de espera, el breaker pasa a semiabierto y deja pasar unas pocas llamadas de prueba. Si salen bien, se cierra y el tráfico vuelve a la normalidad; si fallan, se abre de nuevo y espera otra vez.

Un breaker funciona mejor con timeouts cortos y una respuesta de reserva útil: un valor en caché, uno por defecto o un mensaje claro de que la función no está disponible. Sin respuesta de reserva también ayuda, porque un error rápido libera los hilos de quien llama para las peticiones que sí pueden salir bien.
