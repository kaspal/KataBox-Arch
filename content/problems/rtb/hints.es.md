## 1

Una subasta espera a sus pujadores, y el más lento decide cuándo acaba la espera. ¿Qué pasa con tu latencia, y con tus Servidores de aplicaciones, el día que un pujador se vuelve lento?

## 2

Una puja que llega cuando la subasta ya se ha cerrado no vale nada, así que no tiene sentido esperarla. Ponle un plazo a la llamada a los pujadores: así un pujador tardío se convierte en una puja perdida, no en una subasta lenta, y el hueco del Servidor de aplicaciones vuelve a quedar libre a tiempo. Después asegúrate de tener huecos suficientes para un pico de tráfico, no solo para una hora normal.

## 3

Pon un AWS ALB delante de Servidores de aplicaciones Go que leen los perfiles de una Caché Redis. Pon el Tiempo límite de llamada de los Servidores de aplicaciones en unos 70 ms, muy por dentro del objetivo de p95 de 100 ms pero por encima de los 30 ms habituales de los pujadores. 8 instancias con una Concurrencia por instancia de 256 dejan margen para el doble del tráfico normal.
