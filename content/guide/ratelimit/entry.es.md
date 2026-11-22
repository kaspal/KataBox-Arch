Un limitador de tasa decide, para cada petición, si quien la envía sigue dentro de lo que tiene permitido. El diseño habitual es una **cubeta de tokens** por cliente: los tokens entran a un ritmo constante hasta un máximo, cada petición se lleva uno, y una petición que encuentra la cubeta vacía se rechaza al momento en vez de llegar a tus servidores.

El ritmo fija lo que un cliente puede enviar a lo largo del tiempo, y el tamaño de la cubeta, la **ráfaga**, fija cuánto puede enviar de golpe tras un rato de calma. Como cada cliente tiene su propia cubeta, una clave que se porta mal solo agota lo suyo, y a los demás se les atiende como siempre.

En el simulador el limitador guarda una cubeta por clave de API. Descartar el tráfico que el problema marca como abusivo cuenta como éxito; descartar la petición de un usuario real cuenta como error. Así que el límite debe quedar por encima de los picos de los clientes reales, pero muy por debajo de una avalancha.
