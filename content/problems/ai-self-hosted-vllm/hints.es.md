## 1

Contar huecos no es contar capacidad. Una GPU con 64 huecos lleva 64 respuestas a la vez, pero ¿cuántos tokens por segundo escribe cuando los 64 están ocupados? Compáralo con los tokens por segundo que necesita tu pico, con el doble de tráfico y con una tarjeta caída.

## 2

Un lote lleno escribe cada respuesta a 40 tokens por segundo, así que una GPU escribe como mucho 64 × 40 = 2560 tokens por segundo. El doble de tráfico pide 40 × 250 = 10 000 tokens por segundo, y algo más, porque las respuestas largas suben la media. Cuatro GPU tendrían que ir casi llenas, y una GPU casi llena pone en cola los mensajes nuevos, así que su tiempo hasta el primer token sube varios segundos. Planifica GPU ocupadas entre la mitad y dos tercios en el peor momento. Y repártelas en varias tarjetas del mismo tamaño: cuando se pierde una tarjeta, las demás deben llevar el pico normal.

## 3

Pon un AWS ALB delante de 4 Servidores de aplicaciones Go. Conéctalos a tres tarjetas vLLM en GPU, cada una con 2 Instancias y los Huecos de lote por GPU en 64. En el Cliente web, pon el Tiempo límite en 30 000 ms, los Reintentos en 2, la Espera entre reintentos en 200 ms y la Variación de la espera en Completa (espera aleatoria).
