## 1

Una sugerencia en una GPU ociosa tarda unos 50 ms, así que un presupuesto de 150 ms parece holgado. ¿Qué pasa con ese tiempo cuando las GPU están ocupadas, cuando el tecleo se multiplica por cuatro y cuando falta una tarjeta? ¿Y de verdad necesita una GPU cada pulsación?

## 2

Dos cosas te mantienen dentro del presupuesto. La primera, el margen: un lote más lleno escribe cada sugerencia más despacio, y uno lleno hace esperar a las peticiones, así que planifica las GPU para que sigan lejos de llenarse con cuatro veces el tráfico, repartidas en tarjetas del mismo tamaño para que la pérdida de una deje suficiente. La segunda, una caché delante del modelo: un prefijo idéntico recibe la misma sugerencia, y cada petición que responde la caché es una que las GPU no ven, también con cuatro veces el tráfico. Compara el precio de una caché con el de las GPU que ahorra.

## 3

Pon un AWS ALB delante de 6 Servidores de aplicaciones Go. Estos leen una Caché Redis con 4 GB de Memoria y un TTL de 3600 s, y si falla llaman a tres tarjetas vLLM en GPU, cada una con 2 Instancias, los Huecos de lote por GPU en 64 y las velocidades del modelo que da el enunciado. En el Cliente web, pon el Tiempo límite en 2000 ms, los Reintentos en 2, la Espera entre reintentos en 20 ms y la Variación de la espera en Completa (espera aleatoria).
