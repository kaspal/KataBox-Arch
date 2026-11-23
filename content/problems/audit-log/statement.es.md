# Registro de auditoría

## El producto

Llevas el registro de auditoría de una empresa de software para negocios. Cada producto que vende informa de lo que hacen sus usuarios: quién inició sesión, quién cambió un permiso, quién exportó un informe. Cada una de esas cosas es un evento de auditoría, y tu servicio lo registra. Después, los equipos de cumplimiento y los clientes buscan en el registro para responder «quién hizo qué, y cuándo». Que un evento llegue a los resultados de búsqueda un poco tarde no le importa a nadie. Lo que nadie acepta es un evento que nunca se registró, así que no se puede perder ninguno.

## Lo que debes cumplir

| Objetivo | Valor |
| --- | --- |
| Tráfico pico | 2000 rps |
| Latencia p95 | 100 ms o menos |
| Errores | 1 % o menos |
| Disponibilidad | 99,9 % o más |
| Presupuesto | 2200 $ al mes |
| Mensajes perdidos | Ninguno |

## El tráfico

- **Registrar un evento de auditoría** (97 %). Un producto envía el evento a un App Server, y el evento acaba en un Search Index. El App Server puede pasárselo a un Stream, y entonces un Worker lo toma del Stream y lo escribe en el índice. Si no hay un Stream conectado, el App Server escribe en el índice él mismo, en línea, y la respuesta espera a esa escritura. La latencia se mide hasta que se confirma el evento.
- **Buscar en el registro** (3 %). Un App Server consulta el Search Index y devuelve los eventos que coinciden.

Un balanceador de carga delante de los App Servers es opcional. En este problema no hay servicios externos.

## Las pruebas

Hay una prueba visible. Puedes ejecutarla tantas veces como quieras:

- **Un día normal**: el tráfico descrito arriba, en su pico.

Hay dos pruebas ocultas más, de las que solo ves el nombre: **Un proceso por lotes lo registra todo de golpe** y **El clúster de búsqueda cae durante 30 s**. Tu diseño tiene que superar las tres.
