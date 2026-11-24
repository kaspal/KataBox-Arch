# Solución comentada

## Adónde va la carga

Las 30 000 consultas por segundo acaban todas en una lectura de la tabla de IP. La tabla no cambia mientras el servicio funciona, así que no hay nada que mantener sincronizado: cualquier copia vale lo mismo que otra. Eso abarata las copias, y las preguntas de verdad son cuántos App Servers necesitas y dónde viven las copias.

Con dos regiones y un DNS que reparte a quien llama entre ellas, cada región lleva unas 15 000 peticiones/s en un día normal. Las pruebas ocultas te quitan esa comodidad. En la hora más ocupada el tráfico se duplica hasta 60 000 peticiones/s, así que cada región recibe 30 000. Cuando se apaga una región, la que queda recibe las 30 000 peticiones/s. En los dos casos, una sola región tiene que poder llevar todo el pico normal.

## El diseño de referencia

Un DNS Route 53, con un TTL de 30 s, reparte a quien llama entre dos regiones, us-east y eu-west. Cada región tiene su propio AWS ALB, Go App Servers con 6 instancias y una Redis Cache con 2 GB de memoria y un TTL de 0, para que las entradas no caduquen nunca. Cada Cache guarda la tabla de IP entera, y cada App Server lee solo de la Cache de su propia región. Cuesta unos 1256 $ al mes: 420 $ de App Servers por región, unos 317 $ de Route 53 (se cobra por consulta, y 30 000 consultas por segundo suman), 25 $ por cada ALB y 24 $ por cada Redis Cache.

En «Un día normal» el p95 es de unos 19,8 ms, sin errores, y los App Servers van ocupados en torno al 28 %.

«La hora más ocupada de la semana» duplica el tráfico a partir del segundo 15, durante 30 segundos. Cada región lleva ahora lo que llevaría una región sola, y el p95 se queda en unos 19,4 ms, sin errores.

«Se apaga una región» deja aislada us-east en el segundo 30, durante el resto de una ejecución de 120 segundos. Esta prueba admite un 8 % de errores, un 92 % de disponibilidad y un p95 de 1000 ms. Falla en torno al 0,83 % de las peticiones, todas en los 2 primeros segundos más o menos: hasta que las comprobaciones de salud del DNS se dan cuenta, la mitad de las peticiones de cada segundo siguen yendo a la región apagada. Después todos van a eu-west, que lleva sola toda la carga, porque cada región se dimensionó para toda ella.

Hay margen para ahorrar. Un diseño con 4 instancias de App Server por región y Caches de 0,5 GB supera todas las pruebas por unas tres cuartas partes del coste de la referencia: la tabla sigue cabiendo, y 4 instancias siguen pudiendo llevar solas toda la carga. El reto de coste te pide llegar al 75 % de la referencia o menos.

## Por qué falla el primer intento

Un primer intento habitual es una sola región, us-east, con un AWS ALB, Go App Servers con 4 instancias y una Redis Cache. Cuesta unos 329 $ al mes y supera «Un día normal» con un p95 de unos 18,3 ms. Pero los App Servers ya van ocupados en torno al 85 %.

En «La hora más ocupada de la semana» el tráfico se duplica y no queda margen: los App Servers se saturan y falla en torno al 27,3 % de las peticiones. En «Se apaga una región» es peor. A partir del segundo 30 falla cada petición, porque no hay otro sitio adonde ir, y la ejecución termina con un 74,9 % de errores.

La idea es esta: un conjunto de datos pequeño y de solo lectura es barato de copiar. Pon una copia completa en memoria en cada región, dimensiona cada región para llevar a todos y deja que un DNS con comprobaciones de salud mueva a quien llama. Perder una región cuesta entonces segundos, no el servicio.

## Lo que el simulador deja fuera

No se simula cómo se renueva la tabla de IP, una carga semanal de un fichero en cada Cache. Tampoco la caché de los resolvers más allá del modelo de TTL: los resolvers reales a veces guardan una respuesta más tiempo del debido. Aquí Route 53 reparte por turnos entre las regiones sanas; no se modelan anycast ni el enrutamiento por latencia, ni la distancia de red entre quien llama y una región, porque quien llama está fuera de las dos regiones.

## Para probar

- Baja el TTL del DNS a 10 s y vuelve a ejecutar «Se apaga una región». Mira cuántas peticiones siguen fallando antes de que quien llama se mueva.
- Deja cada región en 3 instancias de App Server y ejecuta la hora más ocupada. Busca el número más pequeño que sigue superando todas las pruebas.
- Quita la región eu-west y ejecuta «Se apaga una región». Compara los errores con el diseño de dos regiones.
