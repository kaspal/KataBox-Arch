## 1

Cuenta lo que aguanta el modelo: 400 000 tokens por minuto son unos 6700 por segundo, y una pregunta usa unos 620. ¿Cuántas preguntas por segundo pueden llegar al modelo y cuántas llegan? Después pregúntate cuáles de ellas tiene que responder de verdad el modelo.

## 2

Una caché normal solo acierta si la pregunta está escrita exactamente igual, y la gente casi nunca la escribe igual. Una caché que compara las preguntas por su significado responde las casi repetidas, y cuando un tema se hace viral casi todas las preguntas sobre él son casi repetidas. La copia de esa caché es lo que mantiene el modelo por debajo de su límite, así que no puede volver vacía tras un reinicio.

## 3

Usa una tarjeta de Servidor de aplicaciones Go (sus 2 instancias por defecto) detrás del Cliente web, y una tarjeta de Inferencia de modelos API de Claude con sus valores por defecto. Lee de una Caché Redis con Modo "Semántica (casi repetidas)" y 2 GB de memoria, conectada con Replicar a una segunda Caché Redis con los mismos ajustes.
