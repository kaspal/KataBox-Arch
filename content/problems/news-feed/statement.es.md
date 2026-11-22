# Feed de noticias

## El producto

Tienes el feed principal de una red social. La gente lo abre muchas veces al día y espera ver al instante los últimos posts de las cuentas que sigue. Casi todo el trabajo es lectura: un feed se abre muchas más veces de las que alguien publica. Cuando alguien publica, todas las personas que lo siguen deberían encontrar el post en su feed poco después. La mayoría de las cuentas tiene unos pocos seguidores, pero algunas tienen muchísimos.

## Lo que debes cumplir

| Objetivo | Valor |
| --- | --- |
| Tráfico pico | 10 000 peticiones/s |
| Lecturas | 97 % de las peticiones |
| Latencia p95 | 100 ms o menos |
| Disponibilidad | 99,9 % o más |
| Presupuesto | 3500 $ al mes |

## El tráfico

- **Abrir tu feed** (97 %). La petición puede pasar por un componente de borde, como un Load Balancer, y luego llega a un App Server. El App Server puede consultar primero una Cache y, si no encuentra el feed, lee el almacén de datos. Hay 1 millón de usuarios y la frecuencia con la que se abre cada feed sigue una curva Zipf 1,0. Cada página del feed ocupa unos 16 KB.
- **Publicar un post** (3 %). El App Server escribe el post en la base de datos principal y lo publica en un broker; con eso, el autor termina. Un Worker lo recoge y lo escribe en el feed de cada seguidor, ya sea en una Cache o en la base de datos. Un post tiene 10 seguidores de media, pero algunas cuentas tienen unas 50 veces más.

## Las pruebas

Hay una prueba visible. Puedes ejecutarla tantas veces como quieras:

- **Un día normal**: el tráfico descrito arriba, en su pico.

Hay dos pruebas ocultas más, de las que solo ves el nombre: **Todos abren la app a la vez** y **Noticia de última hora**. Tu diseño tiene que superar las tres.
