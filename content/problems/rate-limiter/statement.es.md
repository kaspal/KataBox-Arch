# Limitador de tasa

## El producto

Tienes una API pública con 50 clientes de pago. Cada cliente la llama con su propia clave de API, y cada llamada hace un trabajo pequeño contra tu base de datos. La mayoría de los clientes se portan bien. A veces una clave no: un bucle de reintentos roto, un script que alguien dejó encendido o alguien que quiere ver cuánto aguantas. Tu trabajo es que la API siga rápida para todos los demás cuando eso pase, sin rechazar a los clientes que simplemente tienen mucho trabajo.

## Lo que debes cumplir

| Objetivo | Valor |
| --- | --- |
| Tráfico pico | 3 000 peticiones/s |
| Latencia p95 | 100 ms o menos |
| Disponibilidad | 99,9 % o más |
| Presupuesto | 1 500 $ al mes |

## El tráfico

- **Llamada a la API** (99 %). Llamadas de las 50 claves de clientes, unas 59 peticiones/s por clave en el pico. Cada llamada pasa por al menos un componente de borde, luego por un App Server, que lee y escribe en la base de datos.
- **Llamadas de una clave abusiva** (1 %). Llamadas de una única clave adicional, marcadas como abusivas. Siguen la misma ruta. Bloquearlas cuenta como un éxito; bloquear una llamada de un cliente real cuenta como un error.

Hace falta al menos un componente de borde delante de los App Servers.

## Las pruebas

Hay una prueba visible, que puedes ejecutar tantas veces como quieras:

- **Un día normal**: el tráfico descrito arriba, en su pico.

Hay dos pruebas ocultas más, de las que solo ves el nombre: **Una clave inunda la API** y **Día de lanzamiento de todos los clientes (×2 durante 15 s)**. Tu diseño tiene que superar las tres.
