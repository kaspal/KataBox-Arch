# Chatbot de soporte

## El producto

Tienes una tienda online y vas a añadir un chatbot de soporte a su página de ayuda. Un cliente escribe una pregunta; tu backend la envía, con los artículos de ayuda de la tienda como contexto, a un modelo de lenguaje alojado, y la respuesta vuelve a la ventana del chat palabra a palabra. Tú no ejecutas el modelo: pagas a un proveedor de modelos por token, a través de una clave de API, y el proveedor limita cuánto puede enviar cada clave por minuto. Los clientes juzgan el bot ante todo por lo rápido que aparecen las primeras palabras, y pierden la paciencia con un mensaje de error mucho antes que con una respuesta lenta. Marketing prepara un lanzamiento que traerá varias veces el tráfico habitual, y los proveedores de modelos también tienen días malos.

## Lo que debes cumplir

| Objetivo | Valor |
| --- | --- |
| Tráfico pico | 5 mensajes/s |
| Tiempo hasta el primer token, p95 | 1,5 s o menos |
| Latencia p95 (la respuesta entera) | 12 s o menos |
| Disponibilidad | 99 % o más |
| Errores | 1 % o menos |
| Presupuesto | 150 000 $ al mes |

## El tráfico

- **Enviar un mensaje al chat** (100 %). La petición puede pasar por un balanceador de carga, llega a tus Servidores de aplicaciones y estos llaman a una tarjeta de Inferencia de modelos para obtener una respuesta de chat. Un mensaje envía unos 800 tokens al modelo (la pregunta más el contexto; 1600 en el p95) y recibe unos 300 tokens (600 en el p95).

Una respuesta tarda varios segundos en llegar entera, así que la ventana del chat (la tarjeta Cliente) la espera hasta 30 s: pon el Tiempo límite del Cliente en 30 000 ms.

El nivel de tu cuenta da a cada clave de API (cada tarjeta de Inferencia de modelos) 4000 peticiones y 400 000 tokens por minuto. Antes del lanzamiento puedes pedir al proveedor un nivel más alto, de hasta 1 000 000 de tokens por minuto por clave. Una llamada que pasa del límite de una clave se rechaza con un HTTP 429 y cuenta como un mensaje fallido, salvo que el cliente lo vuelva a intentar. También puedes usar un segundo proveedor que sirve un modelo comparable con sus propios límites. La factura es por token, al mismo precio en todos los proveedores de aquí, así que el número de claves y de proveedores no la cambia.

## Las pruebas

Hay una prueba visible. Puedes ejecutarla tantas veces como quieras:

- **Un día normal**: el tráfico descrito arriba, en su pico.

Hay dos pruebas ocultas más, de las que solo ves el nombre: **Día de lanzamiento: 4 veces el tráfico durante 3 minutos** y **El proveedor del modelo se satura durante 20 s**. Un lanzamiento real dura horas; la prueba dura 3 minutos, lo bastante para agotar lo que se permite en un minuto. La prueba del lanzamiento admite un 5 % de errores y un 95 % de disponibilidad, y la de la saturación un 2 % de errores y un 98 % de disponibilidad. Tu diseño tiene que superar las tres.
