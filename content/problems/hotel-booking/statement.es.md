# Reservas de hotel

## El producto

Tienes una web de reservas de hotel. Los viajeros buscan hoteles en una ciudad para unas fechas, miran los resultados y reservan una habitación. Las búsquedas son la mayor parte del tráfico, y unas pocas búsquedas (una gran ciudad en un puente) se repiten una y otra vez. Las reservas son pocas en comparación, pero tienen que salir bien: una noche de una habitación solo se puede vender una vez. Si dos viajeros reservan la misma habitación para la misma noche, uno de ellos llega y se la encuentra ocupada, así que nunca debe haber una reserva doble. Un resultado de búsqueda puede tener un minuto de antigüedad; lo que decide es la reserva.

## Lo que debes cumplir

| Objetivo | Valor |
| --- | --- |
| Tráfico pico | 5000 rps |
| Latencia p95 | 200 ms o menos |
| Disponibilidad | 99,9 % o más |
| Tasa de errores | 1 % o menos |
| Presupuesto | 2000 $ al mes |

## El tráfico

Las peticiones de los viajeros llegan a tus App Servers, a través de un edge si quieres uno.

- **Buscar hoteles para unas fechas** (98 %). Un App Server lanza la búsqueda en un motor de búsqueda, a través de una caché si añades una. Hay 200 000 búsquedas distintas, y unas pocas búsquedas populares de ciudad y fechas se llevan una gran parte.
- **Reservar una habitación** (2 %). Un App Server lee la disponibilidad de la habitación en la base de datos SQL y después escribe la reserva. El 20 % de las reservas son de una de 10 habitaciones que mucha gente quiere a la vez. Si dos reservas leen «libre» antes de que ninguna escriba, las dos salen bien, y la habitación se vende dos veces. El simulador lo cuenta como un conflicto.

## Las pruebas

Hay una prueba visible. Puedes ejecutarla tantas veces como quieras:

- **Una semana normal**: el tráfico descrito arriba, en su pico.

Hay dos pruebas ocultas más, de las que solo ves el nombre: **Se abren las reservas de vacaciones** y **Todos quieren la última habitación**. Tu diseño tiene que superar las tres.
