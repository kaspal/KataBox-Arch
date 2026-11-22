# Viajes compartidos

## El producto

Tienes una aplicación de viajes compartidos. Cada conductor de turno envía su posición cada pocos segundos, así que el servicio siempre sabe dónde están los coches. Cuando un pasajero pide un coche, el servicio busca los conductores cercanos y elige uno. Los pasajeros son lo que la gente ve, pero los conductores son lo que notan los servidores: los envíos de ubicación superan con mucho a las peticiones de viaje, y cada uno es una escritura. Algunos conductores pasan mucho más tiempo en la calle que otros, así que sus envíos son más frecuentes.

## Lo que debes cumplir

| Objetivo | Valor |
| --- | --- |
| Tráfico pico | 15 000 peticiones/s |
| Latencia p95 | 60 ms o menos |
| Disponibilidad | 99,9 % o más |
| Presupuesto | 25 000 $ al mes |

## El tráfico

- **Un conductor envía su ubicación** (98 %). La petición pasa por un componente de borde, como un Load Balancer, y luego llega a un App Server, que escribe la nueva posición en el almacén de datos. Hay 50 000 conductores y su actividad sigue una curva Zipf 0,8.
- **Un pasajero pide un coche** (2 %). La petición pasa por el borde y luego el App Server lee un índice de conductores cercanos en una Cache, que aquí hace de almacén para ese índice. Hay 2 000 zonas que consultar.

Cada envío es una escritura. Nada de una ubicación merece guardarse en caché para después: el siguiente envío la sustituye.

## Las pruebas

Hay una prueba visible. Puedes ejecutarla tantas veces como quieras:

- **Un día normal**: el tráfico descrito arriba, en su pico.

Hay dos pruebas ocultas más, de las que solo ves el nombre: **Aumento en toda la ciudad** y **Todos en el estadio**. Tu diseño tiene que superar las tres.
