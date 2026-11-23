# Analistas a las 9 de la mañana

## El producto

Tu empresa tiene unos 300 analistas, y todos consultan el mismo almacén de datos Snowflake desde una herramienta de BI. No abren un panel compartido: cada uno escribe sus propias preguntas ad hoc ("ingresos por región de esta campaña", "bajas de los clientes que se dieron de alta en marzo"), así que casi no hay dos consultas iguales. Todos empiezan a las 9 de la mañana, y el almacén tiene que aguantar el ritmo.

El equipo de ciencia de datos trabaja sobre el mismo almacén desde notebooks. Sus consultas de exploración son distintas: cada una recorre todo el histórico. En un día normal son pocas, pero cuando el equipo empieza una investigación nueva, llegan muchas a la vez. Los analistas no deben esperar porque un notebook esté ocupado.

## Lo que debes cumplir

| Objetivo | Valor |
| --- | --- |
| Tráfico pico | 12 consultas/s |
| Latencia p95 | 2 s o menos |
| Disponibilidad | 99,9 % o más |
| Errores | 1 % o menos |
| Presupuesto | 7000 $ al mes, medido en una hora normal de trabajo |

## El tráfico

- **Lanzar una consulta ad hoc desde la herramienta de BI** (99,98 %). La consulta llega al servidor de la herramienta de BI (un Servidor de aplicaciones), que la ejecuta en un Almacén de datos. Cada consulta escanea 0,1 GB. Solo el 5 % repite una consulta anterior, así que la Caché de resultados del almacén responde a pocas.
- **Lanzar una consulta pesada de exploración desde un notebook** (0,02 %, más o menos una cada 7 minutos). La consulta llega a la pasarela de notebooks (un Microservicio), que la ejecuta en un Almacén de datos. Cada consulta escanea 60 GB.

Los analistas y los notebooks esperan hasta 120 s una respuesta: pon el Tiempo límite del Cliente en 120 000 ms.

## El almacén

La empresa usa Snowflake de tamaño X-Small. En la tarjeta del Almacén de datos:

- Las **Instancias** son los clústeres del almacén. Cada clúster ejecuta **Concurrencia por instancia** = 8 consultas a la vez; las demás esperan en la cola del almacén, y cuando la cola se llena, el almacén las rechaza.
- Una consulta tarda la **Sobrecarga por consulta** (300 ms) más su escaneo a la **Velocidad de escaneo por consulta** (1 GB/s): unos 0,4 s una consulta ad hoc y unos 60 s una consulta de exploración.
- La concurrencia, la velocidad de escaneo y la sobrecarga vienen con el tamaño, así que déjalas en 8, 1 GB/s y 300 ms. Para ejecutar más consultas a la vez, añade clústeres.
- Cada clúster cuesta 1460 $ al mes si funciona todo el tiempo. El almacén solo se factura mientras trabaja: tras **Suspensión automática tras** (60 s) sin consultas, deja de facturar hasta que llega la siguiente consulta.

## Las pruebas

El simulador acorta el tiempo: una avalancha que en la vida real dura media hora aquí dura 40 s, y una tarde de exploración dura 2 minutos.

Una prueba es visible. Puedes ejecutarla tantas veces como quieras:

- **Una hora normal de trabajo**: el tráfico de arriba, en el pico.

Hay dos pruebas ocultas más, y solo ves sus nombres: **Las 9 de la mañana: cuatro veces las consultas durante 40 s** y **El equipo de ciencia de datos se pone a explorar todo el histórico**. Tu diseño tiene que superar las tres.
