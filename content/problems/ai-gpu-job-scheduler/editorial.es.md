# Solución comentada

## Adónde va la carga

Aquí el tiempo está reducido: un trabajo de unos 2 s representa uno real de unos 15 minutos, y el límite de 8 s, alrededor de una hora. La forma del problema es la misma a las dos escalas.

Cuenta tiempo de GPU, no trabajos. En el pico, 3 trabajos urgentes por segundo ocupan cada uno una GPU unos 2 s, y 3 trabajos masivos por segundo ocupan cada uno una unos 6 s. Así que en un día normal más o menos una cuarta parte de las GPU ocupadas ejecuta trabajo urgente y tres cuartas partes ejecutan trabajo masivo. Con 4 nodos (32 GPU) el clúster está ocupado alrededor del 80 %: para eso es un clúster compartido, y da abasto.

El día de lanzamiento multiplica por cuatro los trabajos urgentes. Solo el trabajo urgente necesita ahora más o menos tantas GPU como toda la carga normal, unas 25, y con los trabajos masivos encima al clúster se le piden unas 45 GPU de trabajo. 32 GPU no pueden con eso, así que crece un atasco cada segundo que dura el empujón del lanzamiento. Qué trabajos esperan en ese atasco lo decide la cola.

## El diseño de referencia

Un Cliente web llama a 2 Servidores de aplicaciones Go, que publican cada trabajo en una Cola RabbitMQ con "Primero lo urgente (prioridad)" activado y un tiempo de espera de visibilidad de 60 s. Cinco nodos de GPU consumen de ella, cada uno un Worker Celery con 8 Instancias (una por GPU), "Concurrencia por instancia" 1 y un tiempo de servicio de 2000 ms. Cuesta unos 3010 $ al mes: 2800 $ por los cinco nodos de GPU, 140 $ por los Servidores de aplicaciones y 70 $ por la Cola. En un día normal el p95 es de unos 4,2 s sin errores, y los nodos están ocupados entre el 60 % y el 70 % más o menos.

La prioridad es la primera mitad. Cada trabajo urgente pasa por delante de todos los trabajos masivos en cola, así que un trabajo urgente solo espera a que quede libre la siguiente GPU, nunca al montón. Los trabajos masivos usan las GPU que sobran, y el día de lanzamiento son ellos los que esperan: se aceptan enseguida y se ejecutan más tarde, que es lo que significa masivo. "Día de lanzamiento: el cuádruple de trabajos urgentes" tiene un p95 de unos 5,3 s sin errores.

El margen es la segunda mitad. La prioridad no hace aparecer GPU: el trabajo urgente por sí solo tiene que caber en las GPU que te quedan cuando algo se rompe. "Día de lanzamiento, y un nodo de GPU falla durante 80 s" tumba el primer nodo de GPU a los 20 segundos, en día de lanzamiento. Quedan cuatro nodos, 32 GPU para unas 25 GPU de trabajo urgente, ocupadas alrededor del 80 %, y el p95 es de unos 6,0 s sin errores. Aquí importa el tiempo de espera de visibilidad: los trabajos que se ejecutaban en el nodo caído vuelven a la cola 60 s después y se ejecutan de nuevo, así que un puñado de trabajos urgentes termina muy tarde. Son muchos menos del 5 % de los trabajos, así que no mueven el p95. En la vida real, mantén el tiempo de espera de visibilidad por encima de tu trabajo más largo, o un trabajo lento se entregará dos veces (el simulador solo devuelve los trabajos de un nodo caído).

Este problema no tiene reto de coste. La búsqueda encontró un diseño al 80 % del coste solo haciendo que la primera tarjeta de GPU fuera un nodo de una sola GPU, para que el nodo que la prueba tumba sea diminuto. En un clúster real puede fallar cualquier nodo, así que ese diseño no te protegería.

## Por qué falla el primer intento

Un primer intento habitual es el mismo camino con 4 nodos de GPU y una cola FIFO normal (el primero en entrar es el primero en salir). Cuesta unos 2450 $ y supera "Un día de trabajo normal" con un p95 de unos 4,6 s, con los nodos ocupados alrededor del 80 %.

El día de lanzamiento el clúster recibe unas 45 GPU de trabajo para 32 GPU, y la cola crece cada segundo. Cada trabajo urgente nuevo se pone al final de la fila, detrás de todos los trabajos masivos que ya esperan, así que su espera crece con el montón: el p95 de cada segundo pasa de 8 s en pocos segundos y no deja de subir, y la prueba termina con un p95 de unos 49 s. Ningún trabajo falla; los urgentes simplemente esperan a trabajos masivos que nadie espera.

Cuando además falla un nodo, las 24 GPU que quedan van todavía más atrasadas, y el p95 pasa del minuto.

Arreglar solo una mitad no basta. Con prioridad pero todavía 4 nodos, el día de lanzamiento se supera (unos 5,9 s), pero cuando falla un nodo el trabajo urgente por sí solo, unas 25 GPU, ya no cabe en las 24 que quedan: p95 de unos 13,9 s. Con más nodos pero sin prioridad, necesitas GPU para todo el trabajo, urgente y masivo: 7 nodos (4130 $) siguen fallando la prueba del nodo con unos 11,2 s, y 8 nodos la superan pero cuestan unos 4690 $, por encima del presupuesto.

La idea es esta: en un clúster de GPU compartido el trabajo masivo debe llenar todas las GPU libres, así que el clúster funciona casi lleno a propósito. Entonces el orden de la cola decide quién espera, y el trabajo urgente tiene que ir primero. Dimensiona el clúster para el trabajo urgente en su pico con un nodo perdido, y deja que el trabajo masivo absorba el resto.

## Lo que el simulador deja fuera

La escala de tiempo: los trabajos reales duran minutos u horas, y un día de lanzamiento real dura más de dos minutos. Cada nodo de GPU es una tarjeta Worker, con el precio de las instancias de Worker del registro, no con precios reales de GPU, y el simulador no te impide subir la "Concurrencia por instancia" por encima de un trabajo por GPU. Los trabajos masivos en marcha nunca se expulsan (se paran para liberar una GPU para uno urgente), ningún trabajo necesita más de una GPU, y no hay puntos de control, así que un trabajo en un nodo caído vuelve a empezar desde el principio. La memoria de las GPU, los datos que carga cada trabajo, la colocación de los trabajos en los nodos y los repartos justos entre equipos no se simulan. El interruptor "Primero lo urgente (prioridad)" funciona aquí con todos los tipos de Cola; RabbitMQ tiene prioridades de mensaje, mientras que con Amazon SQS usarías dos colas.

## Para probar

- Desactiva "Primero lo urgente (prioridad)" y ejecuta el día de lanzamiento. Mira cómo sube el p95 de cada segundo.
- Quita un nodo de GPU y ejecuta la prueba del fallo de nodo. Compara las GPU del trabajo urgente con las GPU que quedan.
- Pon el tiempo de espera de visibilidad en 10 s y ejecuta la prueba del fallo de nodo. Los trabajos del nodo caído vuelven antes, pero en la vida real cada trabajo de más de 10 s también se entregaría dos veces; el simulador no lo muestra.
