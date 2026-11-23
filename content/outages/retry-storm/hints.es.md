## 1

La lentitud duró diez segundos, pero los errores no pararon cuando terminó. ¿Qué seguía llegando a la base de datos después de los 25 s?

## 2

Cada llamada expirada volvía tres veces más, de golpe: cuatro veces la carga, justo cuando la base de datos no tenía margen. Los reintentos tienen que esperar, repartirse y parar cuando fallan demasiados.

## 3

En el Cliente, deja 3 reintentos pero pon una espera de 200 ms, jitter completo y un presupuesto de reintentos del 10 %. No hace falta cambiar nada más.
