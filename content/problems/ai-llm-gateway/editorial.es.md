# Editorial

## Adónde va la carga

Ocho peticiones por segundo son muy poco tráfico para una pasarela. Lo primero que se acaba es la cuenta del proveedor: 400 000 tokens por minuto, compartidos por diez equipos. Una petición típica son unos 400 tokens de entrada y 60 de salida, unos 460 tokens, así que un día normal gasta unos 8 × 460 × 60 ≈ 221 000 tokens por minuto, algo más de la mitad de la cuenta. El límite de peticiones de la cuenta, 4000 por minuto, queda más lejos: 8 × 60 = 480.

La cuota de cada equipo, 55 000 tokens por minuto, son 55 000 / 460 / 60 ≈ 2 peticiones por segundo. En un día normal cada equipo envía unas 0,8.

En la avalancha, un equipo envía 40 veces su tráfico habitual, unas 32 peticiones por segundo. Junto con las 7,2 de los otros nueve equipos, son unos 39 × 460 × 60 ≈ 1 080 000 tokens por minuto, casi tres veces lo que permite la cuenta. El límite del proveedor es un bucket que guarda la cuota de un minuto, así que aguanta un rato y luego rechaza lo que llegue, sea del equipo que sea.

## El diseño de referencia

Un AWS ALB reparte el tráfico entre dos tarjetas API Gateway, LiteLLM (gateway de LLM). Las dos están conectadas a un único Limitador de tasa compartido, Token bucket en Redis, con un Límite de 2 por segundo y una Ráfaga de 10. El limitador guarda un bucket por clave de API y, como es una sola tarjeta, un equipo tiene los mismos 2 por segundo caigan sus llamadas en la pasarela que caigan. Detrás, un Servidor de aplicaciones Go (2 instancias) llama a una tarjeta de Inferencia de modelos, API de Claude, con sus límites por defecto.

Cuesta unos 47 924 $ al mes. Casi todo son tokens: 47 449 $ de la API de Claude. Las dos pasarelas cuestan 140 $ cada una, el Servidor de aplicaciones 140 $, el limitador 30 $ y el ALB 25 $. La factura de tokens queda algo por encima de 8 peticiones por segundo × un mes × 0,0021 $ por petición típica ≈ 44 000 $, porque las peticiones largas suben la media. Un día normal tiene un p95 de unos 2,2 s para la respuesta entera, un p95 del tiempo hasta el primer token de unos 0,8 s y ningún error.

"Un equipo lanza un script que inunda la pasarela" dura 120 segundos, con ese equipo a 40 veces su tráfico. Su bucket en el limitador deja pasar 2 por segundo y rechaza el resto, lo que cuenta como éxito. El modelo ve unas 9,2 peticiones por segundo, unos 254 000 tokens por minuto, muy por debajo de los 400 000 de la cuenta. No hay errores y el p95 se queda en unos 2,3 s.

"Se pierde un nodo de la pasarela" para la primera tarjeta de pasarela a los 20 segundos, durante 20 segundos. Esta prueba admite un 5 % de errores y un 95 % de disponibilidad. Falla más o menos la mitad de las peticiones en los dos segundos antes de que el balanceador deje de mandar tráfico a la tarjeta muerta; después, la segunda pasarela lo lleva todo. La prueba acaba con un 2,9 % de errores. Con una sola tarjeta de pasarela, todas las peticiones fallan durante los 20 segundos: un 38 % de errores.

Aquí el reto de coste no tiene objetivo: la búsqueda no encontró nada más barato que la referencia por más de unos pocos dólares, porque los tokens son el 99 % de la factura y en un día normal todos los diseños pagan los mismos tokens.

## Por qué falla el primer intento

Un primer intento habitual es el mismo ALB con dos pasarelas LiteLLM, conectadas directamente al Servidor de aplicaciones, sin limitador. Cuesta unos 47 894 $ y supera "Un día normal" igual que la referencia.

En la avalancha, nada se interpone entre el script y la cuenta. El bucket de la cuenta empieza con la cuota de un minuto entero y, a los 30 segundos más o menos, está vacío. A partir de ahí, cerca de la mitad de las peticiones de cada segundo se rechazan con un 429, y los nueve equipos que no hicieron nada mal pierden sus llamadas junto con las del equipo que inunda. La prueba acaba con un 42,5 % de errores. La factura también crece: calculada como un mes, la avalancha cuesta unos 124 852 $, frente a 57 775 $ de la referencia en la misma prueba.

La segunda tarjeta de pasarela hace que el diseño ingenuo supere "Se pierde un nodo de la pasarela": los dos fallos son lecciones distintas.

La idea es esta: un límite compartido del proveedor es un recurso compartido, y el proveedor no distingue a tus equipos. Solo tu pasarela puede hacerlo. Dale a cada inquilino su propio límite allí, calculado a partir de su cuota de tokens, y comprueba que todo el tráfico normal más un inquilino en su límite sigue cabiendo en la cuenta.

## Lo que el simulador deja fuera

El Limitador de tasa cuenta peticiones, no tokens. Un límite por clave real de LiteLLM puede contar tokens, así que un equipo que envía prompts largos se quedaría sin cuota antes; aquí el límite es la cuota de tokens dividida entre una petición típica. En el simulador una tarjeta de pasarela no puede llamar a un modelo, así que un Servidor de aplicaciones hace la llamada por ella. Las demás funciones de LiteLLM (pasar a otro proveedor o modelo, la caché, presupuestos en dólares por clave) no se simulan. El límite del proveedor es un bucket que guarda la cuota de un minuto y se rellena de forma continua; los proveedores reales cuentan a su manera. El streaming no se simula: un Servidor de aplicaciones espera la respuesta entera, y el tiempo hasta el primer token se mide dentro de la llamada al modelo. Las cabeceras Retry-After no se simulan. Los precios son los precios didácticos del registro para la API de Claude (3 $ por millón de tokens de entrada y 15 $ por millón de tokens de salida), y los tokens de una ejecución se cobran como si el pico durara un mes. La calidad de las respuestas no se simula en absoluto.

## Para probar

- Quita el Limitador de tasa y ejecuta la avalancha. Busca el segundo en el que la cuenta se agota y compáralo con 400 000 tokens divididos entre lo que la avalancha se pasa cada minuto.
- Pon el Límite del limitador en 13 y vuelve a ejecutar la avalancha. Luego prueba con 10: supera la prueba de 2 minutos, pero calcula cuánto duraría a ese ritmo el minuto de tokens de la cuenta.
- Borra la segunda tarjeta de pasarela y ejecuta "Se pierde un nodo de la pasarela".
