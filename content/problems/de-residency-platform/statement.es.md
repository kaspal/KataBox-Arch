# Una plataforma de datos, dos jurisdicciones

## El producto

Llevas la plataforma de datos de una app de fitness con usuarios en Estados Unidos y en la Unión Europea. Cada toque en la app manda un evento a tus colectores. Los eventos entran en un stream, un procesador de streams consulta el registro de consentimiento de cada usuario y escribe el evento en una tabla del lago, y los analistas de EE. UU. siguen un panel global construido con agregados.

Los contratos de tus usuarios de la UE dicen dónde se pueden guardar sus datos: **los eventos en bruto de un usuario de la UE, y su registro de consentimiento, solo se pueden guardar en regiones de la UE**, y eso vale también en un mal día, cuando algo conmuta. El equipo legal ha autorizado guardar en EE. UU. los agregados (recuentos por país y día, sin ningún usuario dentro). Los datos de los usuarios de EE. UU. no tienen esa regla.

## Lo que debes cumplir

| Objetivo | Valor |
| --- | --- |
| Tráfico pico | 1000 peticiones/s: 995 eventos y 5 cargas del panel |
| Latencia p95 (un evento: desde que se envía hasta que está en el lago) | 5 s o menos |
| Disponibilidad | 99 % o más |
| Errores | 1 % o menos |
| Eventos perdidos | ninguno |
| Datos de la UE guardados fuera de la UE | nunca (0 infracciones de residencia) |
| Presupuesto | 13 500 $ al mes |

## El tráfico

- **La app de un usuario de EE. UU. envía un evento** (59,5 %).
- **La app de un usuario de la UE envía un evento** (40 %). Sus datos tienen que quedarse en regiones de la UE.
- **Un analista abre el panel global** (0,5 %).

Los dos tipos de evento siguen el mismo camino. La petición puede pasar por una tarjeta DNS y un balanceador de carga y llega a tus colectores (App Servers), que publican el evento (unos 2 KB) en un stream y responden. Un procesador de streams lo consume, lee el registro de consentimiento del usuario en un Almacén NoSQL y escribe el evento en una Tabla lakehouse. La latencia de un evento dura hasta que está en el lago.

Una carga del panel llega a un App Server que consulta un Almacén de datos con los agregados; cada carga escanea unos 2 GB, y un 30 % más o menos repite una consulta reciente. El trabajo que construye los agregados cada hora no forma parte de las pruebas.

## Regiones y residencia

Dibuja un marco de región por cada región de la nube que uses, y dale a la región una **Etiqueta de residencia** en su inspector: `eu` para una región de la UE, `us` para una de EE. UU. Cada vez que un stream, un Almacén NoSQL, una tabla del lago o un almacén de datos guarda o sirve un evento de un usuario de la UE, la prueba mira en qué región está: fuera de una región etiquetada `eu`, es una infracción de residencia. Una tarjeta que no está en ninguna región cuenta como fuera. Los App Servers y los procesadores de streams solo pasan el evento y no se comprueban.

Nada encamina un evento según dónde vive su usuario: una tarjeta DNS manda las apps a las regiones que tiene detrás, sean de usuarios de EE. UU. o de la UE, y una tarjeta conectada a varias otras va turnándose entre ellas. Tu diseño tiene que asegurar que, caiga donde caiga un evento, todo lo que lo guarda está en la UE.

Una llamada de una región a otra tarda unos 70 ms en cada sentido en el simulador.

## Las pruebas

Hay una prueba visible. Puedes ejecutarla tantas veces como quieras:

- **Un día normal**: el tráfico descrito arriba, en su pico.

Hay dos pruebas ocultas más, de las que solo ves el nombre: **Se pierde la región del primario de la tabla de consentimientos** y **Un día de auditoría solo con la UE: falla el primario de la tabla de consentimientos**. En la prueba de auditoría todos los eventos vienen de usuarios de la UE, al mismo ritmo total. Cada prueba oculta dura 3 minutos y admite un 8 % de errores y un 92 % de disponibilidad mientras todo conmuta, pero ninguna infracción de residencia y ningún evento perdido. Tu diseño tiene que superar las tres.
