# Sistema de notificaciones

## El producto

Tienes un servicio que envía notificaciones en nombre de otras apps: avisos push a teléfonos, correos y mensajes de texto. Una app llama a tu Notify API y espera una respuesta rápida; el envío ocurre detrás. Tú no gestionas las redes de push, correo o SMS. Lo hacen proveedores externos, que a veces van lentos y de vez en cuando se caen. Les pase lo que les pase, una notificación que alguien pidió nunca debe desaparecer sin más.

## Lo que debes cumplir

| Objetivo | Valor |
| --- | --- |
| Tráfico pico | 1 500 notificaciones/s |
| Latencia p95 hasta completar | 3 s o menos |
| Disponibilidad | 99,9 % o más |
| Presupuesto | 9000 $ al mes |
| Mensajes perdidos | Ninguno |

## El tráfico

Todas las notificaciones siguen el mismo camino. La Notify API, un App Server, la publica en una cola y termina. Un Worker la toma de la cola y la envía a través del proveedor externo que corresponda. La latencia se mide hasta que el proveedor acepta la notificación.

- **Notificación push** (60 %). El proveedor de push responde en unos 40 ms de mediana.
- **Correo** (30 %). El proveedor de correo responde en unos 150 ms de mediana.
- **SMS** (10 %). El proveedor de SMS responde en unos 300 ms de mediana.

Los tres proveedores tienen colas lentas: de vez en cuando una llamada tarda muchas veces su mediana. Un Worker conserva su hueco durante toda la llamada.

## Las pruebas

Hay una prueba visible. Puedes ejecutarla tantas veces como quieras:

- **Un día normal**: el tráfico descrito arriba, en su pico.

Hay dos pruebas ocultas más, de las que solo ves el nombre: **Se cae el proveedor de correo** y **Un envío masivo de marketing**. Tu diseño tiene que superar las tres.
