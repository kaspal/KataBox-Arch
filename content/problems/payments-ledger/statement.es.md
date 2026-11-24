# Libro mayor de pagos

## El producto

Gestionas el núcleo que mueve el dinero en una app de pagos. Cada transferencia quita una cantidad de una cuenta y se la da a otra, y la app tiene que recordar cada una para siempre. El libro mayor es el registro de todas las transferencias, en orden, y es la fuente de verdad. Los saldos de las cuentas son lo que mira la gente, pero solo son un resumen del libro mayor, que se mantiene al día a partir de él. Cuando la app le dice a un cliente «hecho», esa transferencia no puede desaparecer nunca, aunque una máquina de la base de datos caiga un segundo después.

## Lo que debes cumplir

| Objetivo | Valor |
| --- | --- |
| Tráfico pico | 1000 transferencias/s |
| Latencia p95 hasta completar | 3 s o menos |
| Disponibilidad | 99,9 % o más |
| Presupuesto | 3600 $ al mes |
| Mensajes perdidos | Ninguno |

## El tráfico

Hay un solo tipo de petición, y todas siguen el mismo camino.

- **Hacer una transferencia** (100 %). Las apps de pago llegan a la Ledger API, un App Server, normalmente a través de un Load Balancer. El App Server confirma la transferencia en una base de datos SQL, el libro mayor, y el cliente recibe la respuesta en cuanto termina la confirmación. Después, el libro mayor publica el cambio confirmado en un stream (captura de cambios, CDC). Un Worker lo toma del stream y actualiza los saldos de las cuentas en una base de datos NoSQL. Hay 5 millones de cuentas.

La latencia se mide hasta completar: hasta que el saldo está actualizado, no solo hasta que el cliente recibe la respuesta. Una transferencia confirmada que nunca llega a los saldos cuenta como perdida.

No intervienen proveedores externos.

## Las pruebas

Hay una prueba visible. Puedes ejecutarla tantas veces como quieras:

- **Un día normal**: el tráfico descrito arriba, en su pico.

Hay dos pruebas ocultas más, de las que solo ves el nombre: **Una tormenta de reintentos** y **Falla el primario del libro mayor en plena transferencia**. Tu diseño tiene que superar las tres.
