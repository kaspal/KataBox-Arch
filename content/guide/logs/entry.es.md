Los logs son las líneas que escribe cada servicio mientras trabaja: llegó una petición, falló una consulta, se rechazó un pago. Una pila de logs envía esas líneas desde cada servidor a un solo sitio, las guarda y te deja buscarlas, así que no tienes que entrar en cada máquina para saber qué pasó.

Donde las métricas dicen *que* algo va mal, los logs dicen *qué* ha ido mal, con todo el detalle de cada evento. Son más útiles cuando están **estructurados**, con campos con nombre en lugar de texto libre, y cuando cada línea lleva un id de petición que la une al resto de la historia de esa petición.

Los logs nunca están en la ruta de una petición, así que en el simulador no añaden latencia ni pueden descartar una petición. Solo suman su coste mensual, y un problema puede exigirlos como parte de un diseño listo para producción.
