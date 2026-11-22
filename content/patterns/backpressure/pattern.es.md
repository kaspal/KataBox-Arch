Cada etapa tiene un límite de trabajo que acepta: una profundidad máxima de cola, un grupo fijo de hilos, un tope de peticiones en curso. Cuando llega al límite, la etapa deja de aceptar trabajo, y la anterior tiene que esperar, ir más despacio o rechazar a quien la llama.

La señal viaja hacia arriba hasta llegar a un sitio que pueda gestionarla, normalmente el borde, donde una petición puede rechazarse con un claro "inténtalo más tarde". Es mucho mejor que aceptarlo todo y dejar que la latencia suba para todos los usuarios.

Una cola delante de los workers absorbe ráfagas, pero solo hasta su límite. Dimensiónala para las ráfagas que quieras aguantar, vigila su profundidad y escala los workers según esa profundidad para que la cola se vacíe antes de llenarse.
