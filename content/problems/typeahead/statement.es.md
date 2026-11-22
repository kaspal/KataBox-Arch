# Autocompletado

## El producto

Tienes el cuadro de sugerencias que aparece bajo un campo de búsqueda. Mientras alguien escribe, cada pulsación envía el prefijo actual, y tú respondes con una lista corta de terminaciones probables. La respuesta tiene que llegar antes de la siguiente pulsación, o la lista parece atascada. Todo el mundo empieza con las mismas pocas letras, así que un grupo pequeño de prefijos cortos forma la mayor parte de lo que te piden, mientras que una cola larga de prefijos raros aparece de vez en cuando.

## Lo que debes cumplir

| Objetivo | Valor |
| --- | --- |
| Tráfico pico | 6 000 peticiones/s (una por pulsación) |
| Latencia p95 | 60 ms o menos |
| Disponibilidad | 99,9 % o más |
| Presupuesto | 3 000 $ al mes |

## El tráfico

- **Sugerencias para un prefijo** (100 %). La petición puede pasar por un componente de borde, como un Load Balancer, y luego llega a un App Server. El App Server puede consultar primero una Cache y, si no encuentra el prefijo, pregunta al índice de Search. Hay 200 000 prefijos y su popularidad sigue una curva Zipf 1,1: dominan los prefijos cortos y comunes. La respuesta para un prefijo cambia despacio, así que se puede cachear.

La latencia se mide hasta que las sugerencias vuelven al Client.

## Las pruebas

Hay una prueba visible. Puedes ejecutarla tantas veces como quieras:

- **Un día normal**: el tráfico descrito arriba, en su pico.

Hay dos pruebas ocultas más, de las que solo ves el nombre: **Todos escriben lo mismo** y **El índice se reconstruye**. Tu diseño tiene que superar las tres.
