## 1

En los primeros segundos de una venta, ¿qué está haciendo casi todo el mundo, y qué parte de tu diseño tiene que responderles a todos?

## 2

Sirve las páginas de los eventos desde el borde para que la multitud nunca llegue a tus servidores, y deja que una cola profunda nivele las compras: acepta cada pedido rápido y cobra al ritmo que permita el proveedor de pagos.

## 3

Pon una CDN con un TTL corto (un minuto está bien) delante de un Object Storage con las páginas de los eventos, y haz que reenvíe el pago a un Load Balancer y a unos pocos servidores de la Checkout API. Da a la Queue que hay delante de los Payment Workers un max depth muy grande para que nunca rechace un pedido.
