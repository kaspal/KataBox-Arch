# Solución comentada

## Adónde va la carga

Con 400 peticiones por segundo, 280 son peticiones de restablecimiento y 120 son canjes. Cada petición de restablecimiento acaba en una llamada al proveedor de correo, así que el proveedor recibe 280 llamadas por segundo, muy por debajo de su límite de 600. Quien llama ocupa un hueco en cada llamada: solo con la mediana, 280 × 0,15 s mantienen unas 42 llamadas en curso, y la cola lenta, de hasta 0,8 s, añade más. Nada de esto es trabajo pesado.

Las pruebas ocultas hacen otras preguntas. Mientras el proveedor está caído durante 30 segundos, llegan unos 280 × 30 = 8400 correos de restablecimiento que todavía no tienen adónde ir. Y el 10 % de los canjes cae en 20 tokens que se abren a la vez: 12 por segundo en un día normal, 48 por segundo cuando hay cuatro veces más canjes. Dos canjes del mismo token que se solapan pueden leer los dos «sin usar».

## El diseño de referencia

Un Client pasa por un AWS ALB hasta 2 Go App Servers. Estos guardan los tokens en PostgreSQL con las Escrituras condicionales activadas, así que el canje es una sola sentencia: `UPDATE … SET used = true WHERE token = ? AND used = false`. El que pierde una carrera se rechaza limpiamente, con «este enlace ya se usó». Para una petición de restablecimiento, el App Server publica el correo en una cola RabbitMQ con Reintentos máximos en 10, y 4 instancias Celery de Mailer con concurrencia 32 la consumen y llaman al proveedor. Cuesta unos 775 $ al mes: PostgreSQL 260 $, los Mailers 280 $, los App Servers 140 $, RabbitMQ 70 $ y el ALB 25 $. En un día normal el p95 hasta completar es de unos 608 ms, sin errores.

«El proveedor de correo cae durante 30 s» quita el proveedor a partir del segundo 20, en una ejecución de 120 segundos, y admite un p95 de 60 segundos. Los correos rechazados esperan en la cola y se reintentan tras 1, 2, 4, 8, 16, 32, 32, 32, 32 y 32 segundos: 191 segundos de esperas en total, mucho más que la caída. Salen cuando el proveedor vuelve: el p95 queda en unos 29,5 s, sin errores y sin perder ningún restablecimiento.

«Los escáneres de correo abren cada enlace dos veces» envía cuatro veces más canjes y no admite conflictos, es decir, tokens usados dos veces. Cada carrera por un token tiene un ganador, y el otro canje se rechaza: el p95 queda en unos 498 ms, sin conflictos.

Este problema no tiene reto de coste: la referencia ya es ajustada, y ningún diseño más barato que se encontró lo era lo bastante como para plantear uno.

## Por qué falla el primer intento

Un primer intento habitual usa el mismo borde y 2 Go App Servers que envían ellos mismos el correo, en línea, sin cola (una tarjeta de worker Celery queda en el lienzo, sin conectar), y PostgreSQL sin escrituras condicionales. Cuesta unos 565 $ y supera «Un día normal» con un p95 de unos 452 ms, más rápido que la referencia porque no hay salto por la cola. Pero incluso en ese día normal, 7 tokens se usaron dos veces. La prueba no lo juzga, pero el fallo ya está ahí.

Cuando el proveedor cae, un envío en línea no tiene dónde guardar el correo. La llamada del App Server falla, y la petición también: el 18,5 % de las peticiones de la prueba acaba en error, y 3532 correos de restablecimiento cuentan como perdidos: fallaron sin una cola que los guardara y nada los volverá a intentar. Cuando los escáneres de correo abren cada enlace, el App Server lee el token, ve «sin usar» y escribe, y otro canje hace lo mismo entre medias: 93 conflictos, y la prueba falla.

Aquí hay dos lecciones. Un correo de restablecimiento es un trabajo que puede esperar, y una cola con reintentos convierte una caída en un retraso. Un token de un solo uso necesita comprobar y escribir en una sola sentencia, no «leer y después escribir».

## Lo que el simulador deja fuera

Aquí los tokens no caducan, y no se guardan con hash. No hay un límite por usuario para las peticiones de restablecimiento. No se simulan la entregabilidad del correo ni los filtros de spam. Tampoco se modela el propio escáner de correo: solo aparece como más canjes de los mismos tokens a la vez.

## Para probar

- Vuelve a poner los Reintentos máximos de la cola RabbitMQ en 3 (1 + 2 + 4 = 7 segundos de esperas) y ejecuta de nuevo la caída. ¿Cuántos restablecimientos sobreviven a una caída de 30 segundos?
- Desactiva las Escrituras condicionales de PostgreSQL en la referencia y ejecuta «Un día normal». Busca tokens usados dos veces, aunque la prueba se supere.
- Baja los Mailers a 1 instancia y ejecuta de nuevo la caída. Mira cuánto tarda en vaciarse el atasco cuando el proveedor vuelve.
