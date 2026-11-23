# Agente de soporte con herramientas

## El producto

Tu tienda online va a añadir un agente de soporte. Un cliente pregunta algo sobre un pedido («¿dónde está mi paquete?», «quiero que me devuelvan el dinero»), y la página de ayuda pasa la petición a tu backend como una tarea. Un worker ejecuta la tarea como un agente: pregunta a un modelo de lenguaje alojado qué hacer, llama a las herramientas que pide el modelo, le da los resultados y repite hasta que el modelo escribe la respuesta, que luego se envía al cliente por correo. Las herramientas son APIs: tu propio sistema de pedidos, el seguimiento de paquetes del transportista y el proveedor de pagos. Nadie mira cómo se ejecuta la tarea, pero los clientes esperan una respuesta en menos de medio minuto, y una tarea no debe perderse nunca.

## Lo que debes cumplir

| Objetivo | Valor |
| --- | --- |
| Tráfico pico | 2 tareas/s |
| Latencia p95 hasta completar | 25 s o menos |
| Disponibilidad | 99 % o más |
| Errores | 1 % o menos |
| Presupuesto | 150 000 $ al mes |
| Tareas perdidas | Ninguna |

## El tráfico

Cada tarea llega a tus Servidores de aplicaciones, quizá a través de un componente de borde, y estos la publican en un broker. Un Worker la toma de ahí y hace él mismo todas las llamadas de la tarea, una detrás de otra:

- **Responder una pregunta sobre un pedido** (80 %): el modelo, luego el sistema de pedidos (`orders`), luego el seguimiento del transportista (`tracking`), y otra vez el modelo para la respuesta.
- **Tramitar un reembolso** (20 %): el modelo, el sistema de pedidos, el modelo, el seguimiento del transportista, el modelo, el proveedor de pagos (`payments`) y el modelo una vez más. Cuatro llamadas al modelo en vez de dos.

Cada llamada al modelo envía unos 1500 tokens (la conversación hasta ese momento más los resultados de las herramientas; 3000 en el p95) y recibe unos 100 (250 en el p95). El modelo es la variante API de Claude de Inferencia de modelos. Deja su Tiempo hasta el primer token y sus velocidades como están: describen al proveedor, no tu diseño. Tu cuenta permite **4000 peticiones** y **2 000 000 de tokens por minuto**: el simulador los aplica a cada tarjeta de API de Claude (el Inspector los muestra, sin poder cambiarlos).

A las herramientas se llega por la red, sin tarjeta propia:

| Herramienta | Mediana | p99 |
| --- | --- | --- |
| `orders` | 80 ms | 400 ms |
| `tracking` | 300 ms | 1500 ms |
| `payments` | 200 ms | 800 ms |

El agente no puede seguir sin el sistema de pedidos ni sin el proveedor de pagos: si falla una de esas llamadas, falla la tarea. Sí puede seguir sin el seguimiento: si la llamada de seguimiento falla, o el Worker deja de esperarla, el agente sigue y responde con lo que dice el pedido. Un Worker conserva su hueco durante toda la tarea, también mientras espera cada llamada.

La latencia se mide hasta completar: desde que llega la tarea hasta que vuelve la última respuesta del modelo.

## Las pruebas

Hay una prueba visible. Puedes ejecutarla tantas veces como quieras:

- **Un día normal**: el tráfico descrito arriba, en su pico, durante 2 minutos.

Hay dos pruebas ocultas más, de las que solo ves el nombre: **La API de seguimiento del transportista tarda 40 s por llamada** y **Semana de devoluciones: cinco veces los reembolsos**. Una semana de devoluciones real dura días; la prueba ejecuta esa mezcla durante 2 minutos. Tu diseño tiene que superar las tres.
