Una saga ejecuta una operación como una secuencia de transacciones locales, una por servicio: reservar el stock, cobrar la tarjeta, contratar el mensajero. Cada paso se confirma por su cuenta. Si un paso posterior falla, la saga ejecuta acciones de compensación para los pasos ya hechos, en orden inverso: devolver el cobro, liberar el stock.

Un orquestador puede dirigir los pasos, llamando a cada servicio y guardando el progreso en su propia tabla, para retomar donde lo dejó tras una caída. El otro estilo es la coreografía, en la que cada servicio reacciona al evento del anterior; no tiene coordinador central, pero el flujo es más difícil de seguir.

Una compensación es una acción nueva, no una vuelta atrás. El cliente puede haber visto el cobro antes de que llegue la devolución. Diseña los pasos para que ese estado intermedio sea aceptable, y pon pronto el paso con más probabilidades de fallar.
