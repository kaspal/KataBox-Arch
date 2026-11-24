# Enrutador de modelos

## El producto

Llevas el asistente de ayuda de un gran producto de software. Los usuarios escriben preguntas y un modelo de lenguaje las responde. La mayoría son fáciles ("¿cómo restablezco mi contraseña?", "¿dónde está el botón de exportar?"), y un modelo pequeño las responde lo bastante bien. Más o menos una de cada cinco es difícil: un registro de errores largo, o una pregunta que necesita varios manuales a la vez. Esas necesitan un modelo grande. Hoy todas las preguntas van al modelo grande, a través de una API alojada, y finanzas se ha fijado en la factura.

Un enrutador delante de tus servicios ya etiqueta cada pregunta como fácil o difícil y la envía al servicio que toca. Tú decides a qué modelo llama cada servicio.

## Lo que debes cumplir

| Objetivo | Valor |
| --- | --- |
| Tráfico pico | 10 preguntas/s |
| Tiempo hasta el primer token, p95 | 2,5 s o menos |
| Latencia p95 (la respuesta entera) | 30 s o menos |
| Disponibilidad | 99,9 % o más |
| Errores | 1 % o menos |
| Presupuesto | 220 000 $ al mes |

## El tráfico

- **Hacer una pregunta fácil** (80 %). La petición puede pasar por un API Gateway, llega al servicio de Respuestas (tarjetas Servidor de aplicaciones) y este llama a una tarjeta Inferencia de modelos para obtener una respuesta de chat. Una pregunta fácil envía unos 1200 tokens al modelo (la pregunta más artículos de ayuda; 2400 en el p95) y recibe unos 100 tokens (220 en el p95).
- **Hacer una pregunta difícil** (20 %). Igual, pero a través del servicio de Investigación (tarjetas Microservicio). Una pregunta difícil envía unos 3000 tokens (6000 en el p95) y recibe unos 1200 (2400 en el p95).

Las etiquetas del enrutador vienen con el tráfico: las preguntas fáciles siempre llegan a tarjetas Servidor de aplicaciones, y las difíciles a tarjetas Microservicio. Tu cableado decide a qué modelo llama cada una.

Hay dos modelos disponibles:

- **El modelo grande**: una tarjeta API de Claude. Se factura por token y escribe unos 100 tokens por segundo (el valor por defecto de la tarjeta). El nivel de tu cuenta permite 4000 peticiones y 1 500 000 tokens por minuto, y el proveedor no lo subirá este trimestre: el simulador los aplica a cada tarjeta de API de Claude (el Inspector los muestra, sin poder cambiarlos). Una llamada por encima del límite se rechaza con un HTTP 429.
- **El modelo pequeño**: un modelo de pesos abiertos que ejecutas tú mismo en una tarjeta vLLM en GPU. Cada instancia de GPU se factura por mes, sirva lo que sirva, y trabaja en varias respuestas a la vez (sus Huecos de lote por GPU). El modelo pequeño es rápido: pon su Velocidad de salida en 150 tokens por segundo.

Una respuesta difícil tarda muchos segundos en llegar, así que la ventana de ayuda (la tarjeta Cliente) la espera hasta 60 s: pon el Tiempo límite del Cliente en 60 000 ms.

## Las pruebas

Una prueba es visible. Puedes ejecutarla tantas veces como quieras:

- **Un día normal**: el tráfico de arriba, en el pico.

Hay dos pruebas más ocultas, de las que solo ves el nombre: **El doble de preguntas difíciles** y **Un boletín trae el triple de preguntas fáciles**. Un boletín real trae su ola durante horas; la prueba dura 3 minutos, lo bastante para agotar la asignación de tokens de un minuto. Tu diseño tiene que pasar las tres.
