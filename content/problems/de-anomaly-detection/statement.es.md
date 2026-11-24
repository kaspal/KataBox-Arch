# Alertas de anomalías en turbinas

## El producto

Llevas el backend de monitorización de una empresa eólica: 1500 aerogeneradores con 20 sensores cada uno, 30 000 sensores en total (vibración, temperatura de rodamientos, ángulo de las palas, etc.). Cada sensor mantiene abierta una conexión MQTT con tu Broker IoT y envía un punto cada 10 segundos. Cada punto lo puntúa un clasificador de anomalías que entrenó tu equipo, y se guarda con su puntuación en una Base de datos de series temporales; los ingenieros de guardia reciben una alerta a partir de la puntuación guardada. Un rodamiento que falla tiene que levantar su alerta en menos de 1 segundo desde el punto. En la vida real fallan dos cosas. En una tormenta, los sensores pasan a muestreo rápido y envían 5 veces más puntos, justo cuando las anomalías más importan. Y de vez en cuando una actualización de firmware añade una etiqueta que no debería, y el número de series se dispara.

## Lo que debes cumplir

| Objetivo | Valor |
| --- | --- |
| Tráfico pico | 3000 puntos/s y 30 000 sensores conectados |
| Latencia p95 | 1 s o menos, desde el punto hasta su puntuación en la base de datos (la latencia de la alerta) |
| Errores | 1 % o menos |
| Disponibilidad | 99,9 % o más |
| Sensores conectados | 95 % o más en cada segundo |
| Presupuesto | 7600 $ al mes |

## El tráfico

Hay dos tipos de petición:

- **Sensores conectados por MQTT**. 30 000 sensores mantienen una conexión MQTT con un Broker IoT, conectados desde el principio. En este problema tu backend no les envía nada.
- **Puntuar y guardar un punto** (100 % de los puntos, 3000 por segundo). En el simulador, un punto entra por un Servidor de aplicaciones: piensa en él como el puente que saca cada publicación del broker. Desde ahí puede pasar por un stream opcional hasta un consumidor (un Procesador de streams o un worker). Quien atiende el punto llama a una tarjeta de Inferencia de modelos con el Tipo de modelo en Clasificador, y después escribe el punto y su puntuación en una Base de datos de series temporales. Sin stream, el Servidor de aplicaciones hace las dos llamadas él mismo. La latencia es el tiempo desde el punto hasta su escritura en la base de datos (finalización), no la respuesta del Servidor de aplicaciones.

El clasificador lee una ventana corta de características por punto: unos 30 tokens de entrada (percentil 95: 60) y una puntuación de salida. Corre en tus propias GPU con NVIDIA Triton; mantén los ajustes de esa variante (Tipo de modelo Clasificador, 64 Huecos de lote por GPU) y elige las Instancias. La base de datos lleva 300 000 series: 10 por sensor.

Tu equipo usa un único tamaño de nodo para la Base de datos de series temporales. Mantén estos dos ajustes en todas las tarjetas de Base de datos de series temporales, porque es lo que puede hacer el nodo que pagas:

- **Puntos por segundo**: 20 000 por instancia.
- **Límite de series**: 1 000 000 por instancia (el valor por defecto). Las series se reparten por igual entre las instancias. Por encima del límite un nodo escribe cuatro veces más despacio, y por encima del doble del límite rechaza las escrituras.

En este problema no hay servicios externos.

## Las pruebas

Hay una prueba visible. Puedes ejecutarla tantas veces como quieras:

- **Un día normal**: el tráfico de arriba, en pico.

Hay dos pruebas más ocultas, y solo ves sus nombres: **Una tormenta: los sensores envían 5 veces más puntos** y **Una actualización de firmware multiplica las series por 8**. Tu diseño tiene que superar las tres. Una prueba dura 60 segundos, así que en ella una tormenta o un firmware defectuoso duran 20 segundos en lugar de horas.
