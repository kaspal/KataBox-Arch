Un cortafuegos de aplicaciones web (WAF) se coloca delante de tus puntos de entrada públicos y lee cada petición HTTP antes de que llegue a tu código. Compara cada una con un conjunto de **reglas**: patrones de ataque conocidos como la inyección SQL o el cross-site scripting, bots maliciosos, países bloqueados o límites que escribes tú. Una petición que coincide con una regla de bloqueo se rechaza en el borde.

El proveedor mantiene al día los conjuntos de reglas gestionadas, así que te protegen de ataques comunes sin tocar tu aplicación. El precio es un pequeño retraso en cada petición, una factura por petición y el riesgo de que una regla atrape tráfico inofensivo.

En el simulador, un WAF es un salto con capacidad y un breve tiempo de revisión de reglas en cada petición. Los ataques no se simulan, así que cuesta tiempo y dinero pero no cambia quién pasa. Para frenar la avalancha de una sola clave, usa un Limitador de tasa.
