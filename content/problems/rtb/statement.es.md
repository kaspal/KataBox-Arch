# Subasta de anuncios en tiempo real

## El producto

Tienes el servicio de subastas de un ad exchange. Cada vez que una página web o una app tiene un hueco libre para un anuncio, un ad exchange te envía una petición de puja y espera tu respuesta. Consultas lo que sabes del usuario, preguntas a los pujadores cuánto pagarían y devuelves el resultado. El exchange solo te da unos pocos milisegundos: una respuesta que llega tarde no vale nada, y el hueco se lo lleva otro. Que falte una puja de vez en cuando no importa. Una subasta lenta, sí.

## Lo que debes cumplir

| Objetivo | Valor |
| --- | --- |
| Tráfico pico | 20 000 rps |
| Latencia p95 | 100 ms o menos |
| Latencia p99 | 150 ms o menos |
| Tasa de errores | 0,1 % o menos |
| Disponibilidad | 99,9 % o más |
| Presupuesto | 1300 $ al mes |

## El tráfico

Las peticiones llegan de los servidores de los ad exchanges, no de personas.

- **Hacer una subasta de anuncios** (100 %). Un Servidor de aplicaciones lee el perfil del usuario en el almacén de perfiles (una caché), después llama a los pujadores y cierra la subasta con las pujas que tenga. La llamada a los pujadores es opcional: si falla o tarda demasiado, la subasta se cierra igual, solo que sin esas pujas. Hay 5 millones de perfiles de usuario, y a algunos usuarios se les ve mucho más que a otros.

Los pujadores son un proveedor externo. Una sola llamada los representa a todos juntos, así que el pujador más lento marca lo que tarda: unos 30 ms de mediana y 90 ms en el p99. Un Servidor de aplicaciones conserva su hueco durante toda la llamada.

## Las pruebas

Hay una prueba visible. Puedes ejecutarla tantas veces como quieras:

- **Una hora normal**: el tráfico descrito arriba, en su pico.

Hay dos pruebas ocultas más, de las que solo ves el nombre: **Un pujador grande se vuelve lento** y **Horario de máxima audiencia**. Tu diseño tiene que superar las tres.
