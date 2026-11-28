# Solución comentada

## Adónde va la carga

Con 8000 rps, 7600 peticiones por segundo son búsquedas y 400 son escrituras en el índice. Las búsquedas se reparten entre 1 000 000 de consultas distintas, pero no por igual: con Zipf 1,0, la consulta más popular es cerca del 7 % de todas, más de 500 por segundo. Un índice de búsqueda envía cada consulta a la partición que la contiene, así que esa consulta cae siempre en la misma partición, haya las que haya.

Las pruebas ocultas aprietan en dos puntos débiles distintos. Cuando todo el mundo busca lo mismo, el tráfico se triplica a 24 000 rps, 22 800 búsquedas por segundo, y el sesgo sube a Zipf 1,5: la consulta más popular pasa a ser bastante más de un tercio de todas las búsquedas, miles por segundo en una sola partición. Cuando cae un nodo de búsqueda, el primario del índice desaparece para siempre a partir del segundo 20. Lo que solo estaba en esa tarjeta ya no está para responder.

## El diseño de referencia

Un AWS ALB reparte las peticiones entre 8 Go App Servers. Las búsquedas pasan en cache-aside por una Redis Cache con 8 GB de memoria y un TTL de 300 s, delante de una tarjeta Elasticsearch, «search-a», con 6 shards. «search-a» está conectada con Replicar a una segunda tarjeta Elasticsearch, «search-b», también con 6 shards. Las búsquedas se reparten entre las dos tarjetas; las escrituras del índice van al primario. Cuesta unos 2183 $ al mes: 750 $ cada tarjeta Elasticsearch, 560 $ los App Servers, 98 $ Redis y 25 $ el ALB. En «Un día normal» el p95 es de unos 38 ms, sin errores.

Con esta carga, la caché no es opcional. El ajuste mostró por qué: sin caché, incluso con 10 shards y 8 App Servers se fallaba un día normal. La consulta más popular vive en una partición, esa partición se satura y los App Servers se llenan esperándola. Con la caché, las consultas populares se responden desde Redis y nunca llegan a Elasticsearch.

«Cae un nodo de búsqueda» mata el primario del clúster de búsqueda en el segundo 20, para siempre. Esta prueba admite un 3 % de errores y un 97 % de disponibilidad. La referencia termina con cerca de un 2,5 % de errores. Los errores son las escrituras del índice rechazadas hasta que se promociona la réplica, y las búsquedas que estaban en curso en el primario cuando cayó. Los 2 Reintentos del Cliente web recuperan parte de ellos; sin ellos la referencia se queda en un 2,7 % más o menos, demasiado cerca del límite. Las búsquedas siguen funcionando en la réplica.

«Todo el mundo busca lo mismo» triplica el tráfico a partir del segundo 15, durante 30 segundos, con una popularidad de Zipf 1,5, así que una consulta se lleva una gran parte. La caché responde la consulta de moda: el p95 queda en unos 26 ms, con cerca de un 0,04 % de errores. El precio es la frescura: un resultado en caché puede tener unos minutos, y para un buscador eso está bien.

Aquí no hay reto de coste. El mejor diseño más barato que se encontró costaba el 91 % de la referencia, no lo bastante barato para plantear uno. La segunda tarjeta Elasticsearch es una parte grande de la factura, y es el precio de la lección.

## Por qué falla el primer intento

Un primer intento habitual es el mismo diseño sin la réplica: una sola tarjeta Elasticsearch con 6 shards. Cuesta unos 1433 $ al mes y se acepta en «Un día normal», con un p95 de unos 39 ms. También supera «Todo el mundo busca lo mismo», porque la caché se lleva la consulta de moda.

«Cae un nodo de búsqueda» lo rompe. Desde el segundo 20 no queda índice: falla cada escritura en el índice, y también cada búsqueda que necesita el índice. La ejecución termina con cerca de un 13 % de errores, muy por encima del 3 % que admite la prueba.

La idea es esta: dos hábitos distintos. Replica el índice, porque perder un nodo no debe ser una caída del servicio. Y cachea la cabeza de la distribución de consultas, porque las particiones reparten muchas consultas pero no pueden dividir una consulta popular.

## Lo que el simulador deja fuera

No se simulan la ordenación por relevancia, el rastreo ni la construcción del índice. Tampoco se simulan los shards réplica de Elasticsearch dentro de una tarjeta: la propiedad replicas no cambia el resultado, así que aquí la réplica es una segunda tarjeta. Un buscador real envía cada consulta a todas las particiones y junta las respuestas (scatter-gather); aquí una consulta va a una sola partición. Quedan fuera las reglas sobre lo frescos que deben ser los resultados.

## Para probar

- Quita la Redis Cache de la referencia y ejecuta «Un día normal». Después sube Elasticsearch a 10 shards y vuelve a ejecutarla: ¿se sigue llenando la partición de la consulta popular?
- Quita «search-b» y ejecuta «Cae un nodo de búsqueda». Compara en Resultados los errores antes y después del segundo 20.
- Baja el TTL de la caché a 30 s y ejecuta «Todo el mundo busca lo mismo». Fíjate en cuánto más de la consulta de moda llega a Elasticsearch.
