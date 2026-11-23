# Restablecer la contraseña

## El producto

Llevas el flujo de «¿Olvidaste tu contraseña?» de un gran servicio web. Un usuario escribe su dirección de correo, tu servicio guarda un token de restablecimiento de un solo uso y le envía un correo con un enlace. El usuario abre el enlace y pone una contraseña nueva. Que el correo tarde un poco no le importa a nadie. Lo que sí les importa a los usuarios es un correo que nunca llega. Además, un enlace tiene que funcionar exactamente una vez: un token que cambia la contraseña dos veces es un fallo de seguridad.

## Lo que debes cumplir

| Objetivo | Valor |
| --- | --- |
| Tráfico pico | 400 peticiones/s |
| Latencia p95 | 1000 ms o menos |
| Disponibilidad | 99,9 % o más |
| Errores | 1 % o menos |
| Presupuesto | 1300 $ al mes |
| Mensajes perdidos | Ninguno |

## El tráfico

- **Pedir un enlace de restablecimiento** (70 %). La petición puede pasar por un balanceador de carga, llega a tus App Servers y estos escriben un token de restablecimiento en el almacén de datos. Después el correo sale a través del proveedor de correo. Si el App Server lo entrega a una cola, un Worker lo toma de la cola y llama al proveedor. Sin cola, el App Server llama él mismo al proveedor, y la respuesta lo espera. La latencia se mide desde la petición hasta que el proveedor acepta el correo.
- **Abrir el enlace y poner una contraseña nueva** (30 %). La petición puede pasar por un balanceador de carga, llega a tus App Servers y estos leen el token del almacén de datos y después escriben en él: marcan el token como usado y guardan la contraseña nueva. La mayoría de los canjes son de tokens distintos, pero el 10 % cae en uno de 20 tokens que se están abriendo en el mismo momento.

El proveedor de correo responde en unos 150 ms de mediana, con una cola lenta que llega a 800 ms en el p99. Te acepta como mucho 600 llamadas por segundo.

## Las pruebas

Hay una prueba visible. Puedes ejecutarla tantas veces como quieras:

- **Un día normal**: el tráfico descrito arriba, en su pico.

Hay dos pruebas ocultas más, de las que solo ves el nombre: **El proveedor de correo cae durante 30 s** y **Los escáneres de correo abren cada enlace dos veces**. Tu diseño tiene que superar las tres.
