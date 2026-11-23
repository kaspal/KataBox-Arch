## 1

Si el App Server carga cada lote en Snowflake por su cuenta, ¿qué pasa con los lotes que llegan durante los 60 s que está caído? Algo tiene que guardarlos hasta que vuelva. Y cuando vuelve, ¿cuántos lotes están esperando, y a qué ritmo pueden entrar mientras siguen llegando otros nuevos?

## 2

Publica cada lote en un Stream y cárgalo desde ahí con un Conector de datos: una carga fallida se queda en el Stream y se reintenta, así que no se pierde nada. Después dimensiona para la recuperación, no para el día normal. En el pico se acumulan 45 × 60 = 2700 lotes. El atasco solo baja con lo que la tubería carga por encima de los 45 por segundo que siguen llegando: tiempo de vaciado = atasco ÷ (ritmo de recuperación − ritmo en vivo). Para 50 s necesitas unas 45 + 2700 ÷ 50 ≈ 99 cargas por segundo. Snowflake carga unas 8 ÷ 0,3 ≈ 27 por segundo por instancia.

## 3

Conecta los App Servers con una conexión Publish a un Stream de Apache Kafka con 48 particiones, consumido por Kafka Connect con 3 instancias y una concurrencia de 16 (48 huecos), que carga en Snowflake con 4 instancias (32 cargas a la vez, unas 107 por segundo). Mantén las particiones y los huecos del conector por encima de los 32 del almacén, para que el almacén sea el único tope. Pon un AWS ALB y 2 App Servers de Go delante.
