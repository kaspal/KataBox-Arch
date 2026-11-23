# Planificar trabajos en un clúster de GPU compartido

## El producto

Tu equipo de plataforma de ML gestiona un único clúster de GPU compartido para toda la empresa. Los investigadores le envían trabajos a través de una API interna: evaluaciones urgentes que alguien está esperando (hay que comprobar una versión candidata antes de publicarla) y trabajos masivos que nadie espera (barridos de hiperparámetros, ajustes finos programados, pruebas de rendimiento nocturnas). Las GPU son caras, así que el clúster debe estar siempre ocupado: el trabajo masivo llena todas las GPU que el urgente no necesita. Lo que molesta a tus usuarios es un trabajo urgente atascado detrás de un montón de trabajos masivos.

**Tiempo a escala.** Los trabajos reales duran de minutos a horas. El simulador ejecuta dos minutos, así que el tiempo está reducido: un trabajo que aquí tarda unos 2 s en una GPU representa uno de unos 15 minutos, y el límite de 8 s representa "terminado como mucho una hora después de enviarlo".

## Lo que debes cumplir

| Objetivo | Meta |
| --- | --- |
| Envíos en el pico | 6 trabajos/s (3 urgentes, 3 masivos) |
| Latencia p95 (urgentes: desde el envío hasta terminar; masivos: hasta que se acepta el envío) | 8 s o menos |
| Disponibilidad | 99,9 % o más |
| Errores | 1 % o menos |
| Presupuesto | 4500 $ al mes |

## El tráfico

- **Enviar un trabajo de evaluación urgente** (50 %). La petición llega a tus Servidores de aplicaciones, que ponen el trabajo en una Cola. Un Worker lo toma de la Cola y lo ejecuta en una GPU. El tiempo medido va desde el envío hasta que el trabajo termina.
- **Enviar un trabajo masivo (barrido o ajuste fino programado)** (50 %). El mismo camino, pero un trabajo masivo dura el triple en su GPU. Nadie lo espera: el tiempo medido es solo hasta que se acepta el envío (la Cola lo guarda), así que en la práctica el límite de p95 lo deciden los trabajos urgentes (en un día normal, más o menos su 10 % más lento). Los trabajos masivos están marcados como trabajo de fondo, así que una Cola con "Primero lo urgente (prioridad)" activado entrega antes todos los trabajos urgentes.

Un trabajo urgente dura unos 2 s en una GPU en la mediana, con una cola lenta de unos 8 s en el p99; un trabajo masivo dura unos 6 s en la mediana. El clúster está hecho de nodos de 8 GPU cada uno, y una GPU ejecuta un trabajo a la vez. Dibuja cada nodo de GPU como una tarjeta Worker con 8 Instancias (una por GPU), "Concurrencia por instancia" 1 y "Tiempo de servicio" 2000 ms; el simulador no te impide subir la concurrencia, así que déjala en 1. Un trabajo ocupa su GPU hasta que termina.

## Las pruebas

Una prueba es visible. Puedes ejecutarla tantas veces como quieras:

- **Un día de trabajo normal**: el tráfico de arriba, en el pico, durante dos minutos.

Hay dos pruebas más ocultas, y solo ves sus nombres: **Día de lanzamiento: el cuádruple de trabajos urgentes** y **Día de lanzamiento, y un nodo de GPU falla durante 80 s**. Tu diseño tiene que superar las tres.
