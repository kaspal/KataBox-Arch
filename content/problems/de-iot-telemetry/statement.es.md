# Telemetría de una flota IoT

## El producto

Llevas el backend de 25 000 contadores de energía inteligentes. Cada contador mantiene abierta una conexión MQTT con tu Broker IoT y envía una lectura cada 10 segundos. Las lecturas acaban en una Base de datos de series temporales, y el equipo de operaciones las sigue en paneles en directo: una lectura debería aparecer en menos de 2 segundos. Los contadores son baratos y sus redes no son buenas, así que en la vida real pasan dos cosas. Una zona entera pierde la red móvil, y cuando vuelve cada contador sube las lecturas que guardó mientras estaba sin conexión. Y de vez en cuando una actualización de firmware añade a las lecturas una etiqueta que no debería, y el número de series se dispara.

## Lo que debes cumplir

| Objetivo | Valor |
| --- | --- |
| Tráfico pico | 2500 lecturas/s y 25 000 contadores conectados |
| Latencia p95 | 2 s o menos, de la lectura a la base de datos |
| Errores | 1 % o menos |
| Disponibilidad | 99,9 % o más |
| Contadores conectados | 95 % o más en cada segundo |
| Presupuesto | 2600 $ al mes |

## El tráfico

Hay dos tipos de petición:

- **Dispositivos conectados por MQTT**. 25 000 contadores mantienen una conexión MQTT con un Broker IoT, conectados desde el principio. En este problema tu backend no les envía nada.
- **Escribir una lectura** (100 % de las lecturas, 2500 por segundo). En el simulador, una lectura entra por un Servidor de aplicaciones: piensa en él como el puente que saca cada publicación del broker. Desde ahí puede pasar por un stream opcional hasta un worker, y termina como una escritura en una Base de datos de series temporales. Sin stream, la escritura la hace el propio Servidor de aplicaciones. La latencia es el tiempo desde la lectura hasta su escritura en la base de datos (finalización), no la respuesta del Servidor de aplicaciones.

La base de datos lleva 250 000 series: 10 por contador (potencia, tensión, temperatura, etc.).

Tu equipo usa un único tamaño de nodo para la Base de datos de series temporales. Mantén estos dos ajustes en todas las tarjetas de Base de datos de series temporales, porque es lo que puede hacer el nodo que pagas:

- **Puntos por segundo**: 4000 por instancia. Por encima, las escrituras esperan en la cola del nodo, y cuando la cola se llena se rechazan.
- **Límite de series**: 1 000 000 por instancia (el valor por defecto). Las series se reparten por igual entre las instancias. Por encima del límite un nodo escribe cuatro veces más despacio, y por encima del doble del límite rechaza las escrituras.

En este problema no hay servicios externos.

## Las pruebas

Hay una prueba visible. Puedes ejecutarla tantas veces como quieras:

- **Un día normal**: el tráfico de arriba, en pico.

Hay dos pruebas más ocultas, y solo ves sus nombres: **La flota se reconecta tras un corte de red** y **Una actualización de firmware multiplica las series por 10**. Tu diseño tiene que superar las tres.
