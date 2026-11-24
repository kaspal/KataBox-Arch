# Consultas interactivas sobre una tabla de lago en streaming

## El producto

Los analistas de tu producto exploran el comportamiento de los usuarios en notebooks SQL: escriben una consulta sobre los eventos de hoy, miran la respuesta y escriben la siguiente. Solo siguen concentrados si la respuesta llega en unos pocos segundos. Los eventos les llegan por un pipeline en streaming: unos colectores envían lotes de eventos de producto a tu API, un procesador de streams los escribe en una tabla del lago cada pocos segundos, y un almacén de datos SQL lee esa tabla en su sitio. Escribir cada pocos segundos mantiene los datos frescos, pero deja la tabla llena de archivos pequeños, y una consulta tiene que abrir cada archivo que lee.

## Lo que debes cumplir

| Objetivo | Valor |
| --- | --- |
| Tráfico pico | 10 peticiones/s |
| Latencia p95 | 8000 ms o menos |
| Disponibilidad | 99,9 % o más |
| Errores | 1 % o menos |
| Eventos perdidos | 0 |
| Presupuesto | 71 700 $ al mes |

El p95 se calcula sobre todas las peticiones. La mayoría son consultas, así que en la práctica es la espera de los analistas.

## El tráfico

- **Enviar un lote de eventos de producto** (16 %). Un lote de unos 128 KB puede pasar por un balanceador de carga y llega a tus servidores de aplicaciones, que lo publican en un stream. Un procesador de streams lo consume y lo escribe en la tabla del lago. Su latencia se mide hasta que termina la escritura.
- **Lanzar una consulta interactiva sobre el día de hoy** (83,5 %). Un servidor de aplicaciones envía la consulta al almacén de datos, que lee la tabla del lago. La tabla guarda 30 días de eventos, unos 2 GB al día en las columnas que lee una consulta, así que unos 60 GB en total. Una consulta filtra por hoy: un día de los 30. Una de cada cinco consultas repite exactamente una anterior.
- **Lanzar una consulta interactiva sobre un día de rebajas** (0,5 %). El mismo tipo de consulta, mirando atrás al último día de rebajas: ese día guarda 20 GB de eventos, diez veces un día normal, y la tabla unos 78 GB.

El procesador de streams confirma un archivo cada pocos segundos por cada una de sus tareas de escritura, así que los datos nuevos quedan en archivos de unos 4 MB. En un día de rebajas el pipeline usa diez veces más tareas de escritura para no quedarse atrás, así que los archivos siguen siendo de unos 4 MB y hay diez veces más. Abrir un archivo en el almacenamiento de objetos cuesta unos 20 ms, y una consulta abre hasta 8 archivos a la vez. El resto del coste de una consulta es la sobrecarga por consulta del almacén de datos y los datos que lee a su velocidad de escaneo.

Los notebooks esperan hasta 60 s una respuesta: pon el tiempo límite del Cliente en 60 000 ms.

## Las pruebas

Hay una prueba visible. Puedes ejecutarla tantas veces como quieras:

- **Un día normal**: el tráfico descrito arriba, en su pico.

Hay dos pruebas ocultas más, de las que solo ves el nombre: **Un día de rebajas: diez veces los eventos, y los analistas miran el día de hoy** y **Revisión de fin de mes: cuatro veces las consultas**. Tu diseño tiene que superar las tres.
