La principal recibe todas las escrituras y envía sus cambios a una o más réplicas. Cada réplica los aplica en orden y guarda una copia completa de los datos. La app manda las escrituras a la principal y reparte las lecturas entre todas las copias sanas.

La replicación tarda un poco, así que una réplica siempre va algo por detrás. Un usuario que escribe y lee enseguida puede no ver su propio cambio. Donde eso importe, lee los datos de ese usuario desde la principal durante un rato después de que escriba.

Las réplicas también hacen posible el failover. Cuando la principal cae, una réplica asciende y recibe las escrituras. Hasta que termina el ascenso, las escrituras fallan, así que el tiempo de failover forma parte de tu presupuesto de disponibilidad.
