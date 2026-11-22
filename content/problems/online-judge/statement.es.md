# Juez en línea

## El producto

Tienes un juez en línea para practicar programación y celebrar concursos. Un usuario envía una solución, el juez la compila, la ejecuta contra un conjunto de pruebas en un entorno aislado y guarda un veredicto: aceptada, respuesta incorrecta, tiempo límite excedido. Ejecutar un envío es trabajo lento, un par de segundos cada uno. Los usuarios aceptan esperar un poco su veredicto, pero no mucho, y desde luego no que se pierda un envío. La mayor parte del día es tranquila. Los concursos, no.

## Lo que debes cumplir

| Objetivo | Valor |
| --- | --- |
| Tráfico pico | 40 envíos/s |
| Tiempo de finalización p95 | 20 s o menos |
| Disponibilidad | 99,9 % o más |
| Presupuesto | 3 000 $ al mes |

Aquí la latencia se mide hasta que el veredicto está guardado, no hasta que se acepta el envío.

## El tráfico

- **Evaluar un envío** (100 %). La petición pasa por un componente de borde, como un Load Balancer, y luego llega a la Submit API, en un App Server, que pone el envío en una Queue. Los Judge Workers toman envíos de la Queue y los ejecutan. Cada envío tarda unos 2 segundos, y una instancia de Judge Worker ejecuta 8 a la vez. Cuando termina, el Worker escribe el veredicto en la base de datos.

Los envíos siguen llegando haya o no un juez libre. Lo que todavía no puede ejecutarse tiene que esperar en algún sitio.

## Las pruebas

Hay una prueba visible. Puedes ejecutarla tantas veces como quieras:

- **Una tarde tranquila**: el tráfico descrito arriba, en su pico.

Hay dos pruebas ocultas más, de las que solo ves el nombre: **Empieza el concurso** y **Se cae una máquina de evaluación**. Tu diseño tiene que superar las tres.
