# Un asistente para EE. UU. y la UE

## El producto

Gestionas un asistente de redacción con usuarios en Estados Unidos y en la Unión Europea. Un usuario envía un mensaje, el asistente lo responde palabra a palabra, y la conversación se guarda para que el usuario pueda volver a ella. Sirves tú mismo un modelo de pesos abiertos, con vLLM, en GPU que alquilas por meses.

Los contratos de tus clientes de la UE dicen adónde pueden ir sus conversaciones: **la conversación de un usuario de la UE solo puede guardarse y procesarse en regiones de la UE**. Eso cubre el historial que guardas y las GPU que leen el mensaje y escriben la respuesta: también cuenta dónde procesa el modelo. Vale también en un mal día, cuando una base de datos conmuta o se pierde una región.

## Lo que debes cumplir

| Objetivo | Valor |
| --- | --- |
| Tráfico pico | 30 mensajes/s |
| Tiempo hasta el primer token, p95 | 1 s o menos |
| Latencia p95 (la respuesta entera) | 15 s o menos |
| Disponibilidad | 99 % o más |
| Errores | 1 % o menos |
| Historial de la UE guardado, o mensajes de la UE respondidos, fuera de la UE | nunca (0 infracciones de residencia) |
| Presupuesto | 12 000 $ al mes |

## El tráfico

- **Un usuario de EE. UU. envía un mensaje** (60 %).
- **Un usuario de la UE envía un mensaje** (40 %). Su historial y su llamada al modelo deben quedarse en regiones de la UE.

Los dos siguen el mismo camino. La petición puede pasar por una tarjeta DNS y un balanceador de carga, y llega a tus Servidores de aplicaciones. Estos guardan el mensaje del usuario en el historial de la conversación (una escritura en una base de datos), llaman a una tarjeta de Inferencia de modelos para obtener una respuesta de chat y guardan la respuesta (una segunda escritura). Un mensaje envía unos 800 tokens al modelo (la pregunta y la conversación reciente; 2000 en el p95) y recibe una respuesta típica de unos 250 tokens (600 en el p95).

Una respuesta tarda varios segundos en llegar entera, así que la ventana del chat (la tarjeta Cliente) la espera hasta 30 s: pon el Tiempo límite del Cliente en 30 000 ms.

## Regiones y residencia

Dibuja un marco de Región por cada región de la nube que uses, y dale a la Región una **Etiqueta de residencia** en su inspector: `eu` para una región de la UE, `us` para una de EE. UU. Cada vez que una base de datos, una caché o una cola guarda o sirve un mensaje de un usuario de la UE, y cada vez que una tarjeta de Inferencia de modelos responde uno, la prueba mira la Región en la que está la tarjeta: fuera de una Región con la etiqueta `eu`, es una infracción de residencia. Una tarjeta que no está en ninguna Región cuenta como fuera.

Nada enruta por sí solo un mensaje de la UE: una tarjeta DNS reparte a los usuarios entre las regiones que tiene detrás, de EE. UU. y de la UE por igual, y un Servidor de aplicaciones va turnándose entre las tarjetas de Inferencia de modelos a las que está conectado. Tu diseño debe asegurar que, se sirva donde se sirva un mensaje, el historial que escribe acabe en la UE y la GPU que lo responde esté en la UE. Los mensajes de los usuarios de EE. UU. pueden guardarse y responderse en cualquier sitio.

En el simulador, una llamada de una región a otra tarda unos 70 ms en cada sentido.

## Tus GPU y tu base de datos

Usa tarjetas de Inferencia de modelos **vLLM en GPU** y deja sus velocidades como están: el primer token llega tras 200 ms más el tiempo de leer el prompt a 5000 tokens por segundo, y después una respuesta sola se escribe a 80 tokens por segundo. Una GPU escribe hasta **Huecos de lote por GPU** respuestas a la vez, y con el lote lleno cada respuesta se escribe a la mitad de velocidad. Tu modelo y la memoria de sus conversaciones caben como mucho para **64 respuestas a la vez en una GPU**: deja los Huecos de lote por GPU en 64 o menos. Las **Instancias** de una tarjeta de vLLM son sus GPU; cada GPU cuesta 734 $ al mes, y una tarjeta puede fallar entera.

El historial se guarda en PostgreSQL. Tu equipo de plataforma lo gestiona con failover automático, ajustado para promover una réplica 10 s después de perder el primario: pon el **Tiempo de failover** de las tarjetas de Base de datos SQL en 10 s. Las réplicas son tarjetas de Base de datos SQL conectadas al primario con conexiones de tipo Replicar.

## Las pruebas

Hay una prueba visible. Puedes ejecutarla tantas veces como quieras:

- **Un día normal**: el tráfico descrito arriba, en su pico.

Hay dos pruebas ocultas más, de las que solo ves el nombre: **Se apaga la región de la base de datos del historial** y **Un día de auditoría solo con la UE: falla el primario de la base de datos y después la región a la que pasó**. En la prueba de auditoría todos los mensajes vienen de usuarios de la UE, al mismo ritmo total. La primera prueba oculta dura 5 minutos y la de auditoría 7,5 minutos; cada una admite un 8 % de errores y un 92 % de disponibilidad mientras todo conmuta, pero ninguna infracción de residencia. Tu diseño tiene que superar las tres.
