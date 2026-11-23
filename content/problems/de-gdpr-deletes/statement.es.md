# Borrados por derecho de supresión

## El producto

Tu app guarda datos de sus usuarios en tres sitios: una tabla Apache Iceberg en el data lake (cada evento que han generado), un almacén Snowflake (las tablas que consultan los analistas) y una tabla DynamoDB (su perfil). La ley de privacidad da a cada usuario el derecho a que se borren esos datos. La ley permite hasta un mes; tu empresa promete 24 horas, y tiene que cumplirlo en cada petición, no en promedio.

Un usuario pulsa «Borrar mis datos». El Servidor de aplicaciones publica una petición de borrado en una cola y responde enseguida. Un Worker toma la petición, busca dónde están los datos del usuario y los borra de cada almacén por turno: la tabla del lago, el almacén de datos y el almacén NoSQL. Una petición termina solo cuando el último almacén la ha borrado.

**El tiempo está a escala.** Una prueba dura uno o dos minutos, así que aquí el plazo de 24 horas se juega como **30 segundos** desde el clic hasta que termina el último almacén. La regla que se mantiene es la misma a cualquier escala: cuando los borrados llegan más rápido de lo que los terminas, el atasco crece, y en cuanto es más viejo que el plazo, todas las peticiones que van detrás llegan tarde.

## Lo que debes cumplir

| Objetivo | Valor |
| --- | --- |
| Tráfico pico | 20 peticiones de borrado/s |
| Plazo | cada petición terminada en 30 s (p99), desde el clic hasta que el último almacén la ha borrado |
| Disponibilidad | 99,9 % o más |
| Errores | 0,1 % o menos |
| Peticiones perdidas | ninguna |
| Presupuesto | 6500 $ al mes |

## El tráfico

- **Borrar mis datos** (100 %). El cliente web llama a tus Servidores de aplicaciones, que publican en una cola. Un Worker consume la petición y hace tres escrituras, una detrás de otra: en una Tabla lakehouse, en un Almacén de datos y en un Almacén NoSQL. Cada petición se mide hasta que termina la tercera escritura.

Lo que cuesta cada paso aquí:
- El Worker dedica su tiempo de servicio por defecto, 100 ms, a encontrar los registros del usuario, y mantiene ocupado su hueco mientras espera cada escritura.
- Una escritura en la Tabla lakehouse tarda unos 50 ms. Una Tabla lakehouse acepta mil escrituras a la vez, así que allí las escrituras nunca esperan sitio.
- Una escritura en el Almacén de datos (un DELETE por filas) tarda unos 300 ms, y un Almacén de datos ejecuta «Concurrencia por instancia» consultas a la vez en cada instancia (8 por defecto). Snowflake cobra una instancia por el tiempo que está en marcha, y un almacén que nunca descansa 60 s nunca se suspende.
- Una escritura en DynamoDB tarda unos pocos milisegundos.

Cuando falla un paso de un Worker, la cola vuelve a intentar la petición tras 1, 2, 4, 8, 16 y luego 32 segundos, hasta sus «Reintentos máximos» (3 por defecto). Una petición que falla en todos los reintentos va a una Cola de mensajes fallidos si hay una conectada a la cola; si no, se pierde. Un borrado perdido es una promesa rota: los datos de ese usuario se quedan.

## Las pruebas

Hay una prueba visible. Puedes ejecutarla tantas veces como quieras:

- **Un día normal**: el tráfico de arriba, en el pico, durante un minuto.

Hay dos pruebas más ocultas, y solo ves sus nombres: **Una noticia sobre privacidad se hace viral** y **Las reescrituras de la tabla del lago se ralentizan**. Tu diseño tiene que pasar las tres.
