Un limitador se coloca delante del servicio y lleva un contador por cliente, normalmente por clave de API o por usuario. La forma habitual es un token bucket: el cubo de cada cliente se rellena a un ritmo constante hasta un máximo, cada petición gasta un token, y una petición que encuentra el cubo vacío se rechaza con una respuesta de "demasiadas peticiones".

El ritmo de relleno fija el límite a largo plazo y el tamaño del cubo fija qué ráfaga se permite, así que los usuarios normales con clics a ráfagas pasan y un script que machaca la API queda cortado. Rechazar pronto, antes de hacer trabajo real, es lo que hace barato el límite.

Con varios nodos limitadores los contadores deben compartirse, a menudo en un almacén rápido en memoria, o cada nodo aplica su propia parte del límite. Indica a los clientes cuánto esperar antes de reintentar, para que los clientes bien hechos se retiren en lugar de reintentar al momento.
