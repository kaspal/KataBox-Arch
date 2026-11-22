Las trazas distribuidas siguen una sola petición mientras recorre un sistema. El primer servicio le da a la petición un **id de traza**, y cada servicio al que llama registra un **span**: qué hizo, cuándo empezó y cuánto tardó. El backend de trazas une los spans en una sola línea de tiempo, así que puedes ver el viaje completo de la petición.

Su punto fuerte es responder "¿en qué se fue el tiempo?" cuando una petición cruza muchos servicios. Las métricas muestran que la latencia de cola es alta; una traza muestra que el tiempo se fue esperando una llamada lenta tres saltos más abajo.

Guardar todas las trazas es caro, así que la mayoría de sistemas guardan solo una **muestra**. Las trazas nunca están en la ruta de una petición, así que en el simulador no añaden latencia ni pueden descartar una petición. Solo suman su coste mensual, y un problema puede exigirlas como parte de un diseño listo para producción.
