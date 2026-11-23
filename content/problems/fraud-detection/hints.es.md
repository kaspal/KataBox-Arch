## 1

El modelo de fraude es una llamada en mitad del pago. ¿Qué le pasa a un pago, y a la API que lo retiene, si esa llamada de repente tarda varias veces más de lo normal?

## 2

Una llamada a un modelo en línea necesita un presupuesto de latencia. La puntuación es opcional: cuando llega tarde, pueden decidir las reglas y el pago sigue adelante. Así que deja de esperar al modelo pasado un tiempo fijo, bastante por debajo del p95 de 60 ms, y deja margen para la lectura de la caché. Sin límite, un modelo lento hace lento cada pago, y las solicitudes que esperan llenan la API hasta que falla.

## 3

Servidores de socio → un AWS ALB → 6 instancias de API Go con un Tiempo límite de llamada de 40 ms. La API lee las características de una caché Redis (Memoria de 16 GB) y llama a una tarjeta de Inferencia de modelos NVIDIA Triton con 2 instancias y el Tipo de modelo en Clasificador. Para los eventos, la API publica en un stream Apache Kafka con 24 Particiones, que consume Apache Flink con 2 instancias y escribe las características en el mismo Redis.
