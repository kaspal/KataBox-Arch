# Autocompletado de código mientras escribes

## El producto

Construyes el autocompletado de un editor de código que usan los desarrolladores de una gran empresa. Mientras alguien escribe, el editor envía a tu backend el código anterior al cursor (el prefijo) y muestra en gris el resto de la línea; el tabulador lo acepta. Una sugerencia solo sirve si aparece antes de la siguiente pulsación, así que todo el viaje de ida y vuelta tiene un presupuesto de unos 150 ms. El código no puede salir de la cuenta en la nube de la empresa, así que ejecutas tú mismo un modelo de código pequeño de pesos abiertos, con vLLM, en GPU que alquilas por meses. Cerca de una fecha de entrega todo el mundo escribe a la vez, y los servidores de GPU también fallan.

## Lo que debes cumplir

| Objetivo | Valor |
| --- | --- |
| Tráfico pico | 1500 sugerencias/s |
| Latencia p95 | 150 ms o menos |
| Disponibilidad | 99 % o más |
| Errores | 1 % o menos |
| Presupuesto | 7000 $ al mes |

## El tráfico

- **Sugerir el resto de la línea** (100 %). La petición puede pasar por un balanceador de carga, llega a tus Servidores de aplicaciones, puede responderse desde una caché (cache-aside) y, si no, va a una tarjeta de Inferencia de modelos para obtener una sugerencia. Una petición envía al modelo unos 800 tokens de código (2000 en el p95) y recibe una sugerencia corta de unos 10 tokens (30 en el p95).

El mismo prefijo vuelve una y otra vez: un desarrollador borra un carácter y lo vuelve a escribir, y el mismo código repetitivo (los imports, las primeras líneas de una prueba, la cabecera de un bucle) se escribe en toda la empresa. En una hora hay unos 15 millones de prefijos distintos, y unos pocos se escriben mucho más que el resto (una popularidad Zipf de 0,6). La sugerencia de un prefijo idéntico se puede reutilizar, y una sugerencia en caché ocupa más o menos 1 KB.

El editor espera una sugerencia como mucho 2 s. El editor puede reintentar una petición fallida (los Reintentos de la tarjeta Cliente) tras una espera corta.

## Tus GPU

Usa tarjetas de Inferencia de modelos **vLLM en GPU** para tu modelo de código pequeño, y pon sus velocidades en cada tarjeta: **Tiempo hasta el primer token** 15 ms, **Velocidad del prompt** 50 000 tokens por segundo y **Velocidad de salida** 500 tokens por segundo. El primer token llega tras 15 ms más el tiempo de leer el prompt; después, una sugerencia sola se escribe a 500 tokens por segundo.

vLLM agrupa en lotes: una GPU escribe hasta **Huecos de lote por GPU** sugerencias a la vez, pero cada sugerencia del lote frena a las demás. Con el lote lleno, cada una se escribe a la mitad de velocidad. Una petición que encuentra todos los huecos ocupados espera. Tu modelo y su memoria de trabajo caben como mucho para **64 sugerencias a la vez en una GPU**: deja los Huecos de lote por GPU en 64 o menos.

Una tarjeta de vLLM es un grupo de servidores de GPU que se despliega junto (un grupo de nodos, un mismo despliegue); sus **Instancias** son sus GPU, y cada una guarda su propia copia del modelo. Un grupo puede fallar entero, por ejemplo tras una mala actualización de drivers. Cada GPU cuesta 734 $ al mes.

## Las pruebas

Una prueba es visible. Puedes ejecutarla tantas veces como quieras:

- **Un día normal**: el tráfico de arriba, en el pico.

Hay dos pruebas ocultas más, y solo ves sus nombres: **Una hora punta: cuatro veces más tecleo** y **Se pierde una tarjeta de GPU**. Una hora punta real dura una hora; la prueba dura un minuto con cuatro veces el tráfico, tiempo suficiente para que se note cualquier cola. La prueba de la GPU admite un 2 % de errores y un 98 % de disponibilidad. Tu diseño tiene que pasar las tres.
