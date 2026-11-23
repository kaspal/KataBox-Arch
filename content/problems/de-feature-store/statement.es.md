# Almacén de características para ML

## El producto

Tu empresa tiene modelos de aprendizaje automático que hacen predicciones sobre los usuarios mientras usan la app: qué recomendar, si una sesión parece de riesgo. Cada predicción necesita las características del usuario, una fila de unos 2 KB de números sobre ese usuario (compras de esta semana, tiempo desde la última visita, etc.). Tú llevas el almacén de características (feature store): el servicio que entrega esas características.

Tiene tres clientes. Los servidores de modelos leen las características de un usuario en cada predicción, y necesitan la respuesta rápido. La app envía eventos (un clic, una compra), y un trabajo de streaming convierte cada evento en características actualizadas de ese usuario, que los servidores de modelos deberían ver casi al momento. Y los trabajos de entrenamiento de los científicos de datos leen grandes lotes de características históricas del almacén de datos, donde también acaban todas las características. Los modelos se ejecutan en otro sitio y no forman parte de este problema.

## Lo que debes cumplir

| Objetivo | Valor |
| --- | --- |
| Tráfico pico | 1000 peticiones/s |
| Latencia p95 (respuestas a las lecturas, y fin de las actualizaciones) | 50 ms o menos |
| Disponibilidad | 99,9 % o más |
| Errores | 0,1 % o menos |
| Actualizaciones perdidas | 0 |
| Presupuesto | 4000 $ al mes |

## El tráfico

- **Leer las características de un usuario para una predicción** (80 %). La petición puede pasar por un balanceador de carga y llega a tus App Servers. Estos pueden mirar antes en una caché (cache-aside: en un fallo, la respuesta del almacén llena la caché), y después leen la fila del usuario de un almacén NoSQL o del almacén de datos. Hay 1 000 000 de usuarios, y los más activos hacen la mayoría de las lecturas (una popularidad Zipf de 1,2).
- **Actualizar las características de un usuario con un evento** (19,8 %). Un App Server publica el evento en un Stream. Un procesador de streams lo consume y escribe las nuevas características del usuario en el almacén en línea: un almacén NoSQL, o una caché en la que se escribe directamente. Esta latencia se mide hasta completar: desde el evento hasta que el almacén en línea tiene las nuevas características. Es lo frescas que están las características. Los eventos siguen la misma popularidad que las lecturas.
- **Leer características para entrenar** (0,2 %). Un trabajo de entrenamiento pide al servicio de características offline, un microservicio, que lanza una consulta en el almacén de datos que recorre unos 0,5 GB de histórico.

El almacén de datos (Snowflake por defecto) dedica unos 300 ms a cada consulta antes de leer ningún dato, más el recorrido: unos 0,5 s más para una lectura de entrenamiento, a 1 GB/s. Cada instancia ejecuta 8 consultas a la vez («Concurrencia por instancia»); el resto espera en su cola. Snowflake solo cobra una instancia mientras está en marcha, más su tiempo de suspensión automática.

Un Stream entrega un registro por partición cada vez, y un hueco del procesador de streams retiene un registro hasta que termina su escritura en el almacén en línea. Cuántos registros hay en curso a la vez es lo menor entre las particiones del Stream y los huecos del procesador de streams (instancias × concurrencia por instancia).

El trabajo diario que carga las características en el almacén de datos, y la primera copia de las características de cada usuario en el almacén en línea, ocurren antes de que empiecen las pruebas. Las pruebas no los incluyen.

## Las pruebas

Hay una prueba visible. Puedes ejecutarla tantas veces como quieras:

- **Un día normal**: el tráfico descrito arriba, en su pico.

Hay dos pruebas ocultas más, de las que solo ves el nombre: **Unas rebajas: el triple de tráfico durante 30 s** y **El procesador de streams se ralentiza durante 30 s: las características deben llegar igualmente en 1 s**. Tu diseño tiene que superar las tres.
