## 1

S3 estuvo lento 20 segundos, pero las confirmaciones de pedidos llegaron tarde durante más de un minuto después de que se recuperara. ¿Detrás de qué esperaba cada confirmación nueva?

## 2

La cola es FIFO (el primero en entrar es el primero en salir), y la mayor parte del montón es el relleno de miniaturas, que nadie está esperando. El grupo está ocupado al 90 %, así que solo tiene una décima parte de su potencia para despachar el montón. Deja que los mensajes urgentes se salten los de fondo, y dale a los workers margen para vaciarla.

## 3

En la cola, activa la prioridad (el relleno de miniaturas es trabajo masivo, así que las confirmaciones van primero) y usa 3 workers en vez de 2 para que el montón se vacíe incluso en una hora con más tráfico.
