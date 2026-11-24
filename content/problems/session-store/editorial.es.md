# Solución comentada

## Adónde va la carga

Con 20 000 peticiones/s, unas 18 000 son cargas de página que leen una sesión y unas 2000 son inicios de sesión que escriben una. 2 millones de sesiones de unos 2 KB son unos 4 GB, así que un Redis de 8 GB las guarda todas con espacio de sobra. Redis responde en una fracción de milisegundo, así que el tiempo se va en los App Servers: 20 000 peticiones × 10 ms ≈ 200 huecos ocupados a la vez.

La velocidad es fácil. Lo difícil es que todas las peticiones dependen del almacén de sesiones.

## El diseño de referencia

Un Load Balancer (AWS ALB, round robin) reparte las peticiones entre dos tarjetas de App Servers Node.js, Web A y Web B, con 5 instancias cada una y la concurrencia por defecto de 64: 640 huecos, 320 por tarjeta. Las dos leen y escriben sesiones en una Cache Redis de 8 GB con un TTL de 86 400 s, así que una sesión dura un día. Un segundo Redis del mismo tamaño está conectado a ella con un enlace Replicate. Cuesta unos 920 $ al mes, con un p95 de cerca de 31 ms.

«Se cae un servidor de aplicación» tumba Web A en el segundo 20, para siempre. Admite un 3 % de errores, un 97 % de disponibilidad y un p95 de 100 ms. El Load Balancer sigue mandando la mitad de las peticiones a la tarjeta caída hasta que los health checks lo detectan, entre 2 y 4 segundos después, y esas peticiones fallan: cerca del 1,7 % de la ejecución. Después, los 320 huecos de Web B llevan toda la carga.

«Falla el primario del almacén de sesiones» tumba el primario de Redis en el segundo 20, para siempre, en una ejecución de 120 segundos, y admite un 5 % de errores. Las lecturas ya se reparten entre las dos copias, así que, cuando los health checks apartan las cargas de página del primario caído, la réplica las responde. Los inicios de sesión fallan hasta que la réplica pasa a primario, tras el tiempo de conmutación por defecto de Redis, 5 segundos. En total falla cerca del 1,2 % de las peticiones.

## Por qué falla el primer intento

Un primer intento habitual usa las mismas dos tarjetas de App Servers, pero el Load Balancer enruta por hash de clave, así que cada usuario se queda pegado a una tarjeta, y hay un solo Redis sin réplica. Cuesta unos 823 $ al mes y supera «Un día normal».

También supera «Se cae un servidor de aplicación», con cerca de un 1,7 % de errores, igual que la referencia: el enrutado pegajoso ni ayuda ni estorba aquí, porque las sesiones viven en Redis en los dos casos. Donde falla es en la prueba de conmutación. Sin réplica no hay nada que promover, así que desde el segundo 20 hasta el final de la ejecución fallan todas las peticiones. Son 100 de 120 segundos: cerca de un 83 % de errores.

La lección: sacar las sesiones de los App Servers solo cambia de sitio el punto único de fallo, salvo que el propio almacén tenga una copia que pueda tomar el relevo.

## Lo que el simulador deja fuera

El primer intento clásico guarda las sesiones en la memoria de cada App Server y usa enrutado pegajoso para que cada usuario caiga siempre en el servidor que tiene la suya. Cuando ese servidor cae, sus usuarios pierden la sesión. El simulador no modela el estado dentro de los App Servers, así que no puede mostrar ese fallo. Este problema prueba la parte que sí puede medir: el almacén de sesiones y su conmutación.

## Para probar

- Borra la réplica y ejecuta «Falla el primario del almacén de sesiones». Después vuelve a ponerla y baja el tiempo de conmutación.
- Junta las dos tarjetas de App Servers en una sola de 10 instancias y ejecuta «Se cae un servidor de aplicación».
- Cambia el Load Balancer de la referencia a hash de clave y ejecuta las dos pruebas ocultas. Las cifras apenas cambian.
