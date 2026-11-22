# Feature flags

## El producto

Tienes un servicio de feature flags. Los servicios de otros equipos le preguntan, muchas veces por cada página que sirven, si un flag está activado: mostrar el nuevo proceso de pago, encender la búsqueda en beta, ocultar un botón roto. Cada respuesta tiene que ser rápida, porque quien llama la espera. De vez en cuando alguien cambia un flag y espera que el cambio llegue a todos los servicios en segundos, no en minutos. Y si la base de datos que guarda los flags tiene un mal momento, los servicios que preguntan no deben empezar a fallar con ella.

## Lo que debes cumplir

| Objetivo | Valor |
| --- | --- |
| Tráfico pico | 60 000 peticiones/s |
| Lecturas | 99,9 % de las peticiones |
| Latencia p95 | 30 ms o menos |
| Disponibilidad | 99,9 % o más |
| Presupuesto | 2500 $ al mes |
| Lecturas obsoletas | 3 % o menos |

Una lectura es obsoleta cuando devuelve un valor anterior al último cambio del flag.

## El tráfico

- **Evaluar un flag** (99,9 %). La petición puede pasar por un componente de borde, como un Load Balancer, y luego llega a un App Server, la Flags API. El App Server puede consultar primero una Cache y, si no, lee el almacén de datos. Hay 20 000 flags de 1 KB más o menos cada uno, y su popularidad sigue una curva Zipf 1,1: un grupo pequeño de flags se lleva la mayoría de las lecturas.
- **Cambiar un flag** (0,1 %). El App Server escribe el valor nuevo en la base de datos principal. Los cambios se reparten por igual entre los 20 000 flags.

El App Server conserva su hueco mientras llama a la Cache y a la base de datos, así que una base de datos lenta ocupa sus huecos durante más tiempo.

## Las pruebas

Hay una prueba visible. Puedes ejecutarla tantas veces como quieras:

- **Un día normal**: el tráfico descrito arriba, en su pico.

Hay dos pruebas ocultas más, de las que solo ves el nombre: **Una tormenta de cambios** y **La base de datos de configuración cae 20 s**. Tu diseño tiene que superar las tres.
