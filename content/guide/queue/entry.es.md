Una cola se sitúa entre la parte del sistema que pide el trabajo y la parte que lo hace. Un productor, normalmente un servidor de aplicaciones, **publica** un mensaje y sigue a lo suyo. Un **consumidor**, normalmente un worker, toma el siguiente mensaje cuando tiene un hueco libre, hace el trabajo y lo confirma. Cada mensaje va a un solo consumidor.

La cola convierte una ráfaga en trabajo pendiente. Cuando el tráfico se dispara, los mensajes esperan en lugar de fallar, y los workers se ponen al día después. El precio es el tiempo: el trabajo termina más tarde, así que el usuario recibe un "aceptado" rápido y el resultado real llega a su ritmo.

Las colas también hacen que los fallos se puedan sobrevivir. Un mensaje cuyo trabajo falla se vuelve a intentar tras una breve espera, y la espera crece con cada intento. Si sigue fallando, pasa a una **cola de mensajes fallidos** para que lo revise una persona o una herramienta, en lugar de desaparecer.
