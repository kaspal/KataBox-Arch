## 1

Hacer un pedido tiene dos mitades: guardarlo y cobrar la tarjeta. ¿Por cuál de las dos tiene que esperar de verdad el cliente, y qué les pasa a tus App Servers mientras el proveedor de pagos se toma su tiempo?

## 2

Guarda el pedido y reserva el stock, luego pasa el cobro a una cola y responde al cliente. Los Workers que hay detrás de la cola llaman al proveedor de pagos, así que un proveedor lento llena la cola en vez de los huecos de los App Servers, y la navegación sigue funcionando. Mantén también la caché delante de las lecturas de productos.

## 3

Conecta la Shop API a una Queue SQS con los reintentos máximos subidos a unos 8 y una Dead-letter Queue detrás, y deja que la consuman Workers Celery con una concurrencia alta, de unos 64: cobrar es esperar a la red. Cuenta los huecos de Worker como pedidos por segundo multiplicados por los segundos que tarda un cobro, y deja margen para la cola lenta. Con el cobro fuera del camino de la petición, bastan unas pocas instancias Go.
