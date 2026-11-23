## 1

Durante el pico las tarjetas estaban ocupadas solo unos dos tercios del tiempo, y aun así falló un tercio de las peticiones. Fíjate en el balanceador: ¿qué les pasa a las demás tarjetas cuando saca una de la rotación?

## 2

El health check interpreta una cola larga como una tarjeta caída. Cada expulsión pasa el tráfico de esa tarjeta al resto, cuyas colas también pasan el límite. Una tarjeta ocupada tiene que responder rápido en vez de encolar, y el balanceador nunca debe retirar demasiadas tarjetas a la vez.

## 3

En cada tarjeta de app pon `shed_queue` a 28 (16 huecos de 100 ms cada uno: menos de 200 ms de espera). En el balanceador pon `min_healthy_pct` a 60. Mantén las seis tarjetas y el timeout de health check de 200 ms.
