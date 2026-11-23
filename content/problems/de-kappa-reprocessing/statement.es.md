# Reprocesar en Kappa

## El producto

Llevas el pipeline de eventos de una app de reparto, construido a la manera Kappa: no hay capa batch. Cada evento de pedido entra una sola vez en un topic de Apache Kafka que guarda 30 días de historia, y un Procesador de streams (un job de Apache Flink) convierte cada evento en una fila de la tabla de servicio en PostgreSQL, que leen los paneles de operaciones. Un evento debería llegar a la tabla en cuestión de segundos.

Un error en el job ha calculado mal los tiempos de entrega durante un mes. En una arquitectura Kappa el arreglo no es un job batch aparte: despliegas el código corregido y vuelves a pasar por él los eventos retenidos, desde hace 30 días hasta ahora, mientras los eventos en vivo siguen llegando. El reproceso reescribe las filas del mes; nadie espera por ningún evento reprocesado en concreto, pero las filas en vivo tienen que seguir al día todo el tiempo. La forma obvia de lanzar el reproceso es rebobinar el job en vivo a hace 30 días. Decide si es lo que quieres.

## Lo que debes cumplir

| Objetivo | Valor |
| --- | --- |
| Tráfico pico | 600 eventos/s (300 en vivo, 300 reprocesados) |
| Latencia p95 (ver abajo) | 5 s o menos |
| Disponibilidad | 99,9 % o más |
| Errores | 0,1 % o menos |
| Eventos perdidos | ninguno |
| Presupuesto | 2200 $ al mes |

## El tráfico

- **Procesar un evento en vivo** (50 %). La petición puede pasar por un balanceador de carga y llega a tus Servidores de aplicaciones (la API de ingesta), que la publican en un Stream; el consumidor del Stream la procesa y escribe la fila en la Base de datos SQL. Un evento en vivo se mide por **finalización**: desde que la API lo recibe hasta que su fila está escrita. Eso es lo al día que van los paneles.
- **Reprocesar un evento retenido (reproceso)** (50 %). Empieza en una tarjeta Planificador, que representa al reproceso leyendo de Kafka los eventos retenidos a su ritmo: publica cada uno en un Stream, cuyo consumidor lo procesa con el código corregido y escribe la fila en la Base de datos SQL. Un evento reprocesado se mide por **respuesta**: hasta que su Stream lo ha recibido. Después puede esperar.

Así que el objetivo de p95 se aplica a la finalización de los eventos en vivo y a la confirmación de los reprocesados.

El job está limitado por la CPU: un hueco de Flink procesa unos 200 eventos por segundo (puesto como "Registros por segundo por hueco" = 200), y luego cada evento espera su escritura en PostgreSQL. Una instancia de Flink tiene 4 vCPU y ejecuta 4 huecos ("Concurrencia por instancia" = 4); más huecos por instancia no procesarían más, así que deja ambos valores como están y añade instancias o tarjetas de Procesador de streams. El Stream entrega un evento por partición a la vez.

Los eventos reprocesados son los mismos eventos que los en vivo, leídos otra vez: nada los marca, así que "Primero lo urgente (prioridad)" en un Stream no puede distinguirlos.

Prepárate para perder una tarjeta de Procesador de streams (un clúster de Flink entero) en cualquier momento.

## Las pruebas

Hay una prueba visible. Puedes ejecutarla tantas veces como quieras:

- **Una hora normal, con el reproceso a su ritmo**: el tráfico descrito arriba, en su pico.

Hay dos pruebas ocultas más, de las que solo ves el nombre: **El pico de la tarde, con el reproceso acelerado a 6 veces su ritmo** y **Se pierde un procesador de streams durante el reproceso**. El reproceso entero dura días; una prueba ve uno o dos minutos de él. Tu diseño tiene que superar las tres.
