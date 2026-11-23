# Chat en tus propias GPU

## El producto

Construyes el asistente interno de una gran aseguradora. El personal le pregunta por pólizas, siniestros y procedimientos, y la respuesta vuelve palabra a palabra. Los documentos que lee no pueden salir de la cuenta en la nube de la empresa, así que no llamas a un proveedor de modelos alojados: ejecutas tú mismo un modelo de pesos abiertos, con vLLM, en GPU que alquilas por meses. El personal juzga el asistente ante todo por lo rápido que aparecen las primeras palabras. Dos veces al año, en la época de renovaciones, el tráfico se duplica durante horas, y los servidores de GPU también fallan.

## Lo que debes cumplir

| Objetivo | Valor |
| --- | --- |
| Tráfico pico | 20 mensajes/s |
| Tiempo hasta el primer token, p95 | 1 s o menos |
| Latencia p95 (la respuesta entera) | 15 s o menos |
| Disponibilidad | 99 % o más |
| Errores | 1 % o menos |
| Presupuesto | 8000 $ al mes |

## El tráfico

- **Enviar un mensaje al chat** (100 %). La petición puede pasar por un balanceador de carga, llega a tus Servidores de aplicaciones y estos llaman a una tarjeta de Inferencia de modelos para obtener una respuesta de chat. Un mensaje envía unos 1000 tokens al modelo (la pregunta más los documentos; 2000 en el p95) y recibe una respuesta típica de unos 250 tokens (600 en el p95).

Una respuesta tarda varios segundos en llegar entera, así que la ventana del chat (la tarjeta Cliente) la espera hasta 30 s: pon el Tiempo límite del Cliente en 30 000 ms.

## Tus GPU

Usa tarjetas de Inferencia de modelos **vLLM en GPU** y deja sus velocidades como están: el primer token llega tras 200 ms más el tiempo de leer el prompt a 5000 tokens por segundo, y después una respuesta sola se escribe a 80 tokens por segundo.

vLLM agrupa en lotes: una GPU escribe hasta **Huecos de lote por GPU** respuestas a la vez, pero cada respuesta del lote frena a las demás. Con el lote lleno, cada respuesta se escribe a la mitad de velocidad. Un mensaje que encuentra todos los huecos ocupados espera, y esa espera forma parte de su tiempo hasta el primer token. Tu modelo y la memoria de sus conversaciones caben como mucho para **64 respuestas a la vez en una GPU**: deja los Huecos de lote por GPU en 64 o menos.

Una tarjeta de vLLM es un grupo de servidores de GPU que se despliega junto (un grupo de nodos, un mismo despliegue); sus **Instancias** son sus GPU, y cada una guarda su propia copia del modelo. Un grupo puede fallar entero, por ejemplo tras una mala actualización de drivers. Cada GPU cuesta 734 $ al mes.

## Las pruebas

Hay una prueba visible. Puedes ejecutarla tantas veces como quieras:

- **Un día normal**: el tráfico descrito arriba, en su pico.

Hay dos pruebas ocultas más, de las que solo ves el nombre: **El doble de tráfico durante 2 minutos** y **Se pierde una tarjeta de GPU**. Un pico real de renovaciones dura horas; la prueba dura 2 minutos con el doble de tráfico, lo bastante para que se vea cualquier cola. La prueba de la GPU admite un 2 % de errores y un 98 % de disponibilidad. Tu diseño tiene que superar las tres.
