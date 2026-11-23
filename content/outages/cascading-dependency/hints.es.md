## 1

El checkout puede mostrarse sin recomendaciones, y la ruta ya lo permite. Entonces, ¿por qué falla el checkout cuando el servicio de recomendaciones se cuelga? Fíjate en cuánto tiempo retiene cada checkout su slot del App Server.

## 2

Un fallback solo se ejecuta cuando la llamada termina, y una llamada a un servicio colgado no termina por sí sola: el checkout no tiene timeout de llamada, así que espera hasta que el cliente se rinde. Ponle un límite a la llamada. Luego piensa en el caso lento: un timeout sigue costándole a cada petición su espera completa, salvo que algo deje de llamar a una dependencia que no para de fallar.

## 3

En el App Server del checkout, pon un timeout de llamada de 200 ms y un circuit breaker que se abra con un 20 % de llamadas fallidas durante 30 s. No hace falta cambiar nada más.
