# Solución comentada

## Adónde va la carga

Los Servidores de aplicaciones y el balanceador de carga hacen poco: pasan cada mensaje y esperan la respuesta. El trabajo está en las GPU, y el límite de una GPU son los tokens que puede escribir cada segundo, no el número de huecos que tiene.

Antes del primer token, un mensaje necesita 200 ms más la lectura de su prompt a 5000 tokens por segundo: 0,2 + 1000 / 5000 = 0,4 s para un prompt típico, 0,6 s para uno largo. Ese es el tiempo hasta el primer token (TTFT) cuando nada espera. El límite es 1 s, así que solo quedan unos cientos de milisegundos para esperar en cola.

Después se escribe la respuesta. Una respuesta sola va a 80 tokens por segundo; con B de los 64 huecos ocupados, cada una va a 80 × (1 − 0,5 × B ÷ 64). El compromiso del lote, en números para una GPU:

| Huecos ocupados | Velocidad de cada respuesta | Una respuesta de 250 tokens tarda | La GPU escribe en total |
| --- | --- | --- | --- |
| 16 | 70 tokens/s | 3,6 s | 1120 tokens/s |
| 32 | 60 tokens/s | 4,2 s | 1920 tokens/s |
| 48 | 50 tokens/s | 5,0 s | 2400 tokens/s |
| 64 | 40 tokens/s | 6,3 s | 2560 tokens/s |

Un lote más lleno escribe más tokens en total, y cada respuesta va más lenta. El último cuarto del lote (de 48 a 64) añade solo un 7 % más de tokens por segundo, y hace cada respuesta un 25 % más lenta. Y en 64 no queda nada: el siguiente mensaje espera un hueco, y su espera cuenta en su TTFT.

El pico son 20 mensajes por segundo de unos 250 tokens cada uno: 5000 tokens por segundo, más en la práctica porque las respuestas largas suben la media (los 250 son una respuesta típica, no la media). El doble de tráfico son 10 000 tokens por segundo o más. Una GPU escribe como mucho 2560, así que el doble de tráfico necesita cuatro GPU al 100 % antes de que falle nada más, y una GPU al 100 % pone mensajes en cola.

La prueba de la GPU detiene la primera tarjeta de vLLM desde el segundo 20 hasta el final de una ejecución de 2 minutos. Sus GPU desaparecen, y las demás tarjetas deben llevar el pico normal.

## El diseño de referencia

Un Cliente web con un Tiempo límite de 30 000 ms, 2 Reintentos, una Espera entre reintentos de 200 ms y la Variación de la espera en Completa (espera aleatoria) envía los mensajes a un AWS ALB, que los reparte entre 4 Servidores de aplicaciones Go. Estos llaman por turnos a tres tarjetas vLLM en GPU, cada una con 2 Instancias (2 GPU) y los Huecos de lote por GPU en 64: 6 GPU. Cuesta unos 4709 $ al mes: 4404 $ las 6 GPU, 280 $ los Servidores de aplicaciones y 25 $ el ALB.

En un día normal las GPU están ocupadas en torno a un 22 % (unos 14 de 64 huecos cada una), el TTFT p95 es de unos 0,6 s, el p95 de la respuesta entera de unos 8,8 s, y no hay errores.

«El doble de tráfico durante 2 minutos» duplica el tráfico desde el segundo 20, durante 2 minutos, en una ejecución de 150 segundos. Las seis GPU se reparten 40 mensajes por segundo, unos 6,7 cada una, y están ocupadas en torno a un 50 % a lo largo de la ejecución: más durante el doble de tráfico, con sitio libre en cada lote. Nada espera un hueco: el TTFT p95 se queda en unos 0,6 s, y el p95 de la respuesta entera sube a unos 10,7 s porque cada respuesta se escribe algo más despacio en un lote más lleno. No hay errores.

«Se pierde una tarjeta de GPU» deja 4 GPU para el pico normal, 5 mensajes por segundo cada una: la misma carga por GPU que el primer intento en un día normal. Durante un segundo o dos, hasta que dos comprobaciones de salud seguidas ven la tarjeta caída, los Servidores de aplicaciones siguen enviándole un tercio de los mensajes; esas llamadas fallan al instante, y los reintentos del cliente, tras una espera aleatoria de hasta 200 ms, llegan a una tarjeta que funciona. Falla en torno al 0,12 % de los mensajes, el TTFT p95 es de unos 0,6 s y el p95 de la respuesta entera de unos 9,7 s.

Tres tarjetas de 2 GPU es un tamaño que cubre las dos pruebas: seis GPU para el doble de tráfico, y cuatro que quedan cuando cae una tarjeta. Dos tarjetas de 3 GPU también pasan, al mismo coste (en torno a un 0,41 % de errores cuando se pierde una tarjeta, y un p95 de unos 10,9 s), porque 3 GPU todavía llevan el pico normal.

No hay reto de coste en este problema. La búsqueda de un diseño más barato que supere todas las pruebas no encontró ninguno: una GPU menos, o un lote más pequeño, no supera el doble de tráfico.

## Por qué falla el primer intento

Un primer intento habitual cuenta huecos. Dos tarjetas vLLM de 2 GPU con 64 Huecos de lote por GPU llevan 256 respuestas a la vez; en el pico, 20 mensajes por segundo de unos 4 segundos cada uno son solo unas 80 respuestas en curso, y con el doble de tráfico unas 160. Parece de sobra. Con el mismo cliente, el ALB y 4 Servidores de aplicaciones, cuesta unos 3241 $ al mes, y supera «Un día normal»: las GPU están ocupadas en torno a un 36 %, el TTFT p95 es de unos 0,6 s y el p95 de la respuesta entera de unos 9,5 s, algo más lento que la referencia porque los lotes van más llenos.

Con el doble de tráfico, cada una de las 4 GPU recibe 10 mensajes por segundo. Más respuestas en el lote frenan todas las respuestas, así que cada una ocupa su hueco más tiempo, así que hay aún más respuestas en el lote: los lotes se llenan hasta 64. Entonces una GPU llena escribe 2560 tokens por segundo y se le piden más, y los mensajes de más esperan un hueco. Las GPU están ocupadas en torno a un 87 % a lo largo de la ejecución, lo que significa llenas durante todo el doble de tráfico. A partir de unos 24 segundos dentro de él, el p95 de la respuesta entera pasa de 15 s y sube hasta unos 29 s; el TTFT p95 de la prueba es de unos 21 s, frente a un límite de 1 s. No hay errores (el cliente espera hasta 30 s), pero no cumple ninguno de los dos límites de latencia.

Cuando se pierde una tarjeta, a las 2 GPU que quedan les llegan 10 mensajes por segundo a cada una: la misma sobrecarga, desde los primeros segundos tras la pérdida. La tarjeta que sobrevive está ocupada en torno a un 87 % a lo largo de la ejecución, el TTFT p95 es de unos 18,5 s y el p95 de la respuesta entera de unos 28 s.

Otros primeros intentos fallan de formas que enseñan algo:

- Reducir el lote a la mitad lo empeora, no lo mejora: tres tarjetas de 2 GPU con 32 Huecos de lote por GPU (las GPU de la referencia) fallan con el doble de tráfico con un 45,9 % de errores. Una GPU con 32 huecos escribe como mucho 32 × 40 = 1280 tokens por segundo.
- Cuatro tarjetas de 1 GPU (las 4 GPU del primer intento, repartidas) sobreviven a la pérdida de una tarjeta, porque solo cae una GPU, y aun así fallan con el doble de tráfico.
- Las tarjetas desiguales fallan: tarjetas de 2, 2 y 1 GPU no superan ninguna de las dos pruebas ocultas. Los Servidores de aplicaciones van por turnos entre tarjetas, no entre GPU, así que la tarjeta de 1 GPU recibe tantos mensajes como una de 2 y se llena primero. Haz las tarjetas del mismo tamaño.
- Una sola tarjeta de 6 GPU tiene la capacidad de la referencia y falla la prueba de la GPU con un 86,7 % de errores: cuando cae, no queda nada.

La idea es esta: dimensiona las GPU propias por los tokens que escriben, no por los huecos que tienen. Un lote lleno da el máximo de tokens por segundo y las respuestas más lentas, y no deja sitio, así que un poco más de tráfico se convierte en una cola, y la cola cae sobre el tiempo hasta el primer token. Lleva las GPU ocupadas entre la mitad y dos tercios en tu peor momento, y repártelas en tarjetas iguales para que la pérdida de una deje bastante.

## Lo que el simulador deja fuera

La calidad de las respuestas no se simula: un lote más lleno, u otro modelo, da las mismas respuestas para el simulador. La memoria de la GPU no se simula: el límite de 64 respuestas a la vez en una GPU viene del enunciado, no del simulador. El simulador te dejaría poner los Huecos de lote por GPU en 128, y entonces las 4 GPU del primer intento superan todas las pruebas; una GPU real se quedaría sin memoria para las conversaciones (la caché KV) y pausaría o rechazaría algunas. La forma en que el lote frena las respuestas es una línea recta, una decisión de modelado; las velocidades reales dependen del modelo, de la GPU, de la longitud de los prompts y de la configuración de vLLM. El arranque de un servidor de GPU y la carga del modelo no se simulan, y aquí una tarjeta de Inferencia de modelos no se puede autoescalar. El precio de la GPU es la cifra didáctica del registro (un nodo de GPU a 734 $ al mes, un precio por hora × 730), no un presupuesto real.

## Para probar

- Pon los Huecos de lote por GPU del primer intento en 32 y ejecuta «Un día normal». Mira lo ocupadas que van las GPU.
- Quita la tercera tarjeta de la referencia y da 3 Instancias a cada una de las otras dos. Ejecuta «Se pierde una tarjeta de GPU» y compara sus números con los de la referencia.
- Da 1 Instancia a la tercera tarjeta de la referencia y ejecuta el doble de tráfico. ¿Qué tarjeta se llena primero, y por qué?
