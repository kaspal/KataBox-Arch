# Pastebin

## El producto

La gente pega un bloque de texto, desde un comando de una línea hasta un archivo de log de 1 MB, y recibe un enlace para compartirlo. Cualquiera con el enlace puede leer el paste. La mayoría de las visitas son lecturas, y unos pocos pastes, como uno compartido en un chat concurrido, se leen mucho más que el resto. Las escrituras son menos, pero cada una puede ser grande.

## Lo que debes cumplir

| Objetivo | Valor |
| --- | --- |
| Tráfico pico | 2 000 peticiones/s |
| Lecturas | 85 % de las peticiones |
| Latencia p95 | 150 ms o menos |
| Disponibilidad | 99,9 % o más |
| Presupuesto | 8000 $ al mes |

## El tráfico

- **Leer un paste** (85 %). La petición puede pasar por un componente de borde, como un Load Balancer, y luego llega a un App Server. El App Server puede consultar primero una Cache y, si no encuentra el paste, lee el almacén de datos. Hay 5 millones de pastes y su popularidad sigue una curva Zipf 1,0: algunos se leen mucho más que otros.
- **Crear un paste (hasta 1 MB)** (15 %). El App Server escribe el contenido del paste en un almacén. Cuenta cada creación como un contenido completo de 1 MB.

Tú eliges qué almacén guarda los pastes. Piensa en cómo se comporta cada tipo de almacén con cargas grandes.

## Las pruebas

Hay una prueba visible, que puedes ejecutar tantas veces como quieras:

- **Un día normal**: el tráfico descrito arriba, en su pico.

Hay dos pruebas ocultas más, de las que solo ves el nombre: **Una ráfaga de pastes grandes** y **Caché fría tras un reinicio**. Tu diseño tiene que superar las tres.
