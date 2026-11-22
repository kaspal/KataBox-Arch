# Almacén de sesiones

## El producto

Llevas la capa web de un sitio con mucho tráfico donde casi todo el mundo mantiene la sesión iniciada. Cada página que cargan tiene que saber quiénes son, así que la app busca su sesión antes de pintar nada. Cuando alguien inicia sesión, la app crea esa sesión y la guarda. Si la sesión no aparece, el usuario vuelve de golpe a la página de inicio de sesión, y parece que el sitio se ha roto. Las sesiones tienen que leerse rápido y seguir ahí cuando falla una máquina.

## Lo que debes cumplir

| Objetivo | Valor |
| --- | --- |
| Tráfico pico | 20 000 peticiones/s |
| Lecturas | 90 % de las peticiones |
| Latencia p95 | 50 ms o menos |
| Disponibilidad | 99,9 % o más |
| Presupuesto | 1500 $ al mes |

## El tráfico

Los dos tipos de petición siguen el mismo camino: un componente de borde, que tiene que ser un Load Balancer, después un App Server y después el almacén de sesiones. El almacén de sesiones es una tarjeta Cache usada como el sitio donde viven las sesiones, no como caché delante de otra cosa: todas las peticiones la necesitan y no hay ninguna base de datos detrás. Hay 2 millones de sesiones activas, de unos 2 KB cada una.

- **Cargar una página con sesión** (90 %). El App Server lee la sesión del usuario en el almacén de sesiones.
- **Iniciar sesión** (10 %). El App Server escribe una sesión nueva en el primario del almacén de sesiones.

## Las pruebas

Hay una prueba visible. Puedes ejecutarla tantas veces como quieras:

- **Un día normal**: el tráfico descrito arriba, en su pico.

Hay dos pruebas ocultas más, de las que solo ves el nombre: **Se cae un servidor de aplicación** y **Falla el primario del almacén de sesiones**. Tu diseño tiene que superar las tres.
