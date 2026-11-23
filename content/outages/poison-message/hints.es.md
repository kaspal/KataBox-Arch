## 1

Los pedidos buenos no se volvieron más pesados, y solo uno de cada diez mensajes era malo. Entonces, ¿en qué estaban ocupados los workers hacia el sexto minuto?

## 2

Un mensaje mal formado falla siempre, y la cola le daba 20 reintentos: cada uno costaba a los workers 21 veces el trabajo de un pedido bueno, para nada. Los reintentos son para fallos temporales; un mensaje que sigue fallando hay que apartarlo, no reintentarlo durante minutos y luego descartarlo.

## 3

En la Cola, pon los reintentos en 3, añade una cola de mensajes fallidos (Dead-letter Queue) y conecta la Cola a ella con Publish. No hace falta cambiar nada más.
