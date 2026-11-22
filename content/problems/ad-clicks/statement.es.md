# Agregador de clics en anuncios

## El producto

Te encargas de contar los clics de una red de anuncios. Cada vez que alguien hace clic en un anuncio, el servidor de anuncios que lo mostró te informa del clic, y tú lo sumas a los totales de ese anuncio. Los anunciantes pagan por clic, así que los totales son dinero: pueden ir un poco por detrás durante un rato, pero un clic nunca debe perderse. Los anunciantes y el equipo de facturación consultan de vez en cuando los totales de un anuncio. Unos pocos anuncios se llevan la mayoría de los clics, y los avisos no siempre llegan a un ritmo constante.

## Lo que debes cumplir

| Objetivo | Valor |
| --- | --- |
| Tráfico pico | 10 000 peticiones/s |
| Latencia p95 hasta completar | 30 s o menos |
| Disponibilidad | 99,9 % o más |
| Presupuesto | 8000 $ al mes |
| Mensajes perdidos | Ninguno |

## El tráfico

Los servidores de anuncios llegan a un Click collector, un App Server, normalmente a través de un Load Balancer.

- **Registrar un clic** (99 %). La collector publica el clic en un stream y responde enseguida. Un Worker, el agregador, toma el clic del stream y suma uno al contador del anuncio, que tu diseño guarda en una Cache o en una base de datos. La latencia se mide hasta que el clic queda contado, no hasta que responde la collector.
- **Leer los totales de un anuncio** (1 %). La collector lee el contador del anuncio de donde tu diseño guarde los contadores.

Hay 100 000 anuncios y sus clics siguen una curva Zipf 1,1: un grupo pequeño de anuncios se lleva la mayoría. No interviene ningún proveedor externo.

El límite de 30 s es generoso a propósito. Nadie necesita que un clic se cuente en milisegundos; lo que importa es que se cuenten todos, incluso cuando llegan muchos a la vez.

## Las pruebas

Hay una prueba visible. Puedes ejecutarla tantas veces como quieras:

- **Un día normal**: el tráfico descrito arriba, en su pico.

Hay dos pruebas ocultas más, de las que solo ves el nombre: **Un anuncio se hace viral** y **Los eventos atrasados llegan de golpe**. Tu diseño tiene que superar las tres.
